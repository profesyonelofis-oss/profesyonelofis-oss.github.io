# final100 — İşletme Keşif (masaüstü) + Zeytin Takip (web)

[![Testler](https://github.com/profesyonelofis-oss/final100/actions/workflows/duman-testi.yml/badge.svg)](https://github.com/profesyonelofis-oss/final100/actions/workflows/duman-testi.yml)

Depoda iki ayrı uygulama var:

| Uygulama | Teknoloji | Ne yapar |
|---|---|---|
| **İşletme Keşif** (bu repodaki aktif çalışma) | Rust + egui masaüstü | Web sitesi olmayan işletmeleri bulur; demo, dijital karne ve WhatsApp mesajı hazırlar |
| **Zeytin Takip** | Flask + SQLite web | Satıcı alımları, teslimat fişleri, gider ve komisyon takibi |

## İşletme Keşif — masaüstü uygulaması

Arayüz açık temadır: beyaz paneller, koyu yeşil vurgu (#14663d) ve ince
kenarlar. Uzun süre bakılan bir satış aracı olduğu için kontrast gözetildi;
vurgu zeminler üzerindeki yazı beyazdır.

81 il ve tüm ilçeler, 11 grupta 20 sektör. İşletme taraması OpenStreetMap (Overpass)
üzerinden yapılır; ilçe boş dönerse Google Places yedeği devreye girer ve
ücretsiz kota defterde sayılır. Listeye yalnızca **telefonu olan ve gerçek web
sitesi olmayan** işletmeler girer. OSM kaydında numara yoksa aynı işletme
a Google'da adıyla aranır ve numara oradan tamamlanır. Eşleşme ad benzerliği
(yazım ve Türkçe karakter farkına dayanıklı) ile mesafe puanının birlikte
hesaplanmasıyla yapılır: 300 m'den uzak, adındaki sayı farklı ya da adı çok
kısa kayıtlar eşleşmez. Boş kalan çalışma saati, yıldız ve inceleme sayısı da
aynı eşleşmeden dolar. Eşleşme çıkmazsa
kayıt listeye alınmaz ve 20'lik sınır bu elemeden sonra uygulanır.

Overpass telefonu seyrek taşıdığı için (ölçüm: 100 sitesiz cafenin yalnızca
4'ünde numara var) hibrit taramada numarası eksik kayıt varsa ya da liste
**8 kaydın altında** kalırsa Google'a gidilir; tarama başına en fazla 5 ilçe
için. Tamamlanmış liste önbelleğe yazılır, aynı ilçe tekrar taranınca kota
ikinci kez harcanmaz. Yalnız OSM modunda Google'a hiç gidilmez: numarasız
kayıt listede görünmez. Bulunan işletme için demo sayfası, dijital
görünürlük karnesi ve GitHub Pages yayın linki üretilir.

Demo tek dosyalık bir sayfadır: sabit cam menü, sektöre göre renklenen
animasyonlu hero, mobil menü, kaydırdıkça açılan bölümler ve mobilde sabit
arama düğmesi. Kendi CSS'i ve gömülü fontu vardır; CSS/JS çatısı, CDN veya
ücretli API kullanmaz (ağdan yalnızca gömülü OSM harita çerçevesi gelir).

**Tasarım dili.** Sektöre göre iki tipografi çifti var: restoran, kafe,
pastane, kuaför, klinik, otel ve eğlencede başlık **Instrument Serif**
(editoryal, yüksek kontrastlı), mağaza/kurum/ofis/teknik serviste **Space
Grotesk**. Gövde her yerde **Geist**; Inter bilinçli olarak kullanılmıyor
(AI üretimi sitelerin en belirgin izi). Hareket katmanı: manşet **kelime
kelime** maskesinden açılır, bölüm çizgileri kaydırınca soldan sağa çizilir,
hero sahnesi kırpılarak belirir, imleci izleyen ışık hedefe atlamayıp
arkasından yumuşakça gelir, birincil buton imlece göre birkaç piksel kayar.
Hepsi tek dosyada, kütüphanesiz; `prefers-reduced-motion` açıksa hareket
tamamen kapanır, JS kapalıysa içerik doğrudan görünür.

**Bölümler.** `Hakkında` bölümü, galeriden ayrılan bir kareyle görsellenir
(aynı fotoğraf sayfada iki kez görünmez); hero'da sektör renginden iki yumuşak
ışık lekesi var. Sayfanın en altında **satıcı bandı** durur: “Bu taslağı size
özel hale getirelim” + gönderen numarasına WhatsApp düğmesi. Mesaj işletme
adını taşıdığı için gelen yazışmada hangi demo olduğu karışmıyor; numara
tanımlı değilse bant hiç çizilmez.

**Tasarım referansları.** `sablonlar/` altında lisansı temiz (MIT) şablonlar
ve `sablonlar/sablonlar.json` kaydı durur: Start Bootstrap Creative / One Page
Wonder, Tailwind Toolbox Landing ve dört tek dosyalık küçük işletme şablonu.
Bunlar müşteriye teslim edilmez; bölüm sırası ve düzen fikri alınıp HTML kendi
üreticimizde yazılır. Hangi kaynağın hangi fikri verdiği ve yasaklı lisanslar
`sablonlar/README.md` içinde.
Galeri üç kademeli dolar: (1) işletmenin Google Places fotoğrafları, (2)
işletmenin koordinatından **Google Street View** karesi, (3) aynı sektörün
gömülü örnek kareleri (`varliklar/foto`, Wikimedia Commons, CC/PD). Karelerin
kaynağı sayfada yazar ("Google Street View · işletmenin sokağından" /
"örnek kareler") ve atıflar altbilgiye eklenir. Fiyat listesi hiçbir şablonda
yoktur; fiyat teklifi görüşmede verilir.

**Street View karesi.** Panorama var mı sorusu ücretsiz metadata uçağına
sorulur; panorama yoksa hiç görüntü istenmez (boşa fatura çıkmaz). İki zıt yön
(0° ve 180°) 640x480 olarak alınır — ücretsiz kademenin boyut sınırı. Kareler
`onbellek/streetview` altında tutulur: renk değiştirmek veya önizlemeyi
tazelemek aynı işletme için yeniden ücret ödetmez. Her görüntü isteği arama
kotasıyla **aynı** günlük/aylık sayaçtan düşer; üst şeritteki pil toplam
Google harcamasını gösterir. Street View kapalıysa, panorama yoksa veya bütçe
bittiyse galeri sessizce örnek karelere döner.

Street View'i açmak için Cloud Console'da harita API'leri kütüphanesinden
**Street View Static API**'yi etkinleştir:
<https://console.cloud.google.com/apis/library?filter=category:maps> (anahtar
aynı `GOOGLE_PLACES_API_KEY`). Etkin değilken uygulama çalışmaya devam eder,
yalnızca bu kademe atlanır.

```bash
cargo run                 # derle ve aç
cargo test --all-targets  # 136 test (6 ağ testi varsayılan kapalı)
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

**Satış akışı**

1. İl / ilçe / sektör seç, tara. Liste satın alma potansiyeline göre sıralanır.
2. Karttaki WhatsApp düğmesi mesajı **önce önizler**: metin, karakter sayısı,
   link sayısı ve kalan günlük hak. Yayın linki yoksa önizlemeden önce yayınlanır.
3. "WhatsApp'ı aç ve gönder" sohbetlerini hazırlar; gönderimi siz yaparsınız.
   WhatsApp Desktop kullananlar için **Metni kopyala** düğmesi aynı mesajı panoya alır.
   Kayıtlı numara sabit hatsa (0212/0312...) uygulama "sabit hat" diye işaretler;
   o numarada WhatsApp hesabı olmayabilir.
4. "Gönderdim" işaretlenince kayıt takip kuyruğuna girer; 3 gün sonra
   "Takip kuyruğu" panelinden tek tuşla hatırlatma mesajı hazırlanır.
5. Görüşme notu ve son gönderim tarihi sağdaki işletme detayında durur.
6. İşletme ilgilenince detaydan **paket** (tek sayfa / çok sayfa / bakım),
   **tutar** ve **ödeme durumu** girilir; kısmi ödemede alınan tutar ayrı yazılır
   ve kalan bakiye görünür.
7. Takip panelindeki **ÖDEME BEKLEYEN** bölümü, üç günü dolan tahsilatları
   listeler; "Tahsilat" düğmesi kalan bakiyeyi ve linki içeren mesajı hazırlar.
8. Yanlış ya da süresi geçmiş bir yayın **Yayından kaldır** ile silinir; link 404
   döner ve kayıttaki adres temizlenir.
9. Üst şeritteki **Bugünün listesi**, potansiyele göre sıradaki işletmeleri
   (kara liste, bugün yazılanlar ve telefondan yoksunlar hariç) günlük hak
   kadar listeler; satırdaki **Hazırla** mesaj önizlemesini açar.
10. **Özelleştirme** paneli (üst şeritteki renk düğmesi) demoyu altı yerden
    değiştirir: (a) üç renk — arka plan, yazı, vurgu — ve beş hazır palet;
    (b) **bölüm anahtarları** — galeri, yorumlar, harita, saatler; kapatılan
    bölüm sayfada hiç çizilmez; (c) **metinler** — manşetin iki satırı, marka
    cümlesi, hakkında başlığı ve iki paragrafı, bölüm etiketi/başlığı/alt
    yazısı, dört hizmet satırı (ad + açıklama) ve iki yorum (söz + isim);
    (d) **butonlar** — birincil ve ikincil buton metni; (e) **fotoğraflar** —
    manşet için işletme fotoğrafı / fotoğrafsız tipografik sahne / kendi
    dosyanız, galeri için otomatik kareler, otomatik + eklediklerim, yalnız
    eklediklerim veya kapalı, kare sayısı 1–6, JPEG/PNG/WebP dosyası ekleme
    (panel üzerine sürükle-bırak ya da yol yazma); (f) **köşe biçimi** —
    keskin / dengeli / yumuşak. Boş bırakılan her alan sektör şablonundan
    gelir; hareket ve düzen değişmez. Seçimler `ayarlar.json`'a yazılır
    (fotoğraf yolları saklanır, görüntü değil) ve yeni üretilen demolara
    uygulanır. Paneldeki **Mevcut demolara uygula** demos klasöründeki
    sayfaları güncel ayarlarla **baştan üretir**: metinler, butonlar,
    fotoğraflar, bölümler ve renkler. Bunun için her demo yazılırken kaynak
    işletme kaydı `demos/kaynak/<sayfa>.json` olarak saklanır (koordinat,
    çalışma saatleri ve yorumlar yalnız burada yaşar; defterde yok). Kaynağı
    bulunmayan — elle yapılmış ya da eski — sayfalara yalnız renkler işlenir,
    böylece hiçbiri bozulmaz; günlük kaç sayfanın yenilendiğini yazar. Yayına
    gönderilmiş sayfalar kendiliğinden değişmez, yeniden yayınlamak gerekir.

**Kurallar ve sınırlar**

| Konu | Davranış |
|---|---|
| Günlük gönderim | En fazla **20** mesaj (`GUNLUK_GONDERIM_LIMITI`); sayaç defterde, yeniden başlatınca da sürer; kalan hak WhatsApp ayarında ve önizlemede yazar |
| Mesaj uzunluğu | wa.me bağlantısına sığması için 900 karakterde kırpılır (fazlası "…" ile kesilir) |
| Arama süzgeci | Gerçek web sitesi olan kayıt listeye alınmaz; numarası olmayan kayıt hibritte Google'da aranıp tamamlanır, bulunamazsa listeden çıkar (sabit hat geçerlidir, sosyal medya hesabı site sayılmaz) |
| Google kotası | Üst şeritteki pil kalan günlük/aylık hakkı gösterir (3 saniyede bir tazelenir, tarama bitince hemen); üzerine gelince son kullanım tarihi ve limitler görünür. Hak bitince kırmızı, son 2 gün/20 ay kalınca turuncu |
| Kara liste | "RET" / "Reddetti" işaretlenen işletmeye bir daha mesaj hazırlanmaz |
| A/B denemesi | Varyant sektör içinde dengeli dağıtılır, kayıtta sabit kalır |
| Örneklem | Kol başına 30 gönderimin altında panel "örneklem yetersiz" uyarısı verir |
| Google kotası | `GOOGLE_GUNLUK_LIMIT` (vars. 10) · `GOOGLE_AYLIK_LIMIT` (vars. 200) |
| Defter | `musteri_kayitlari.json`; her yazmadan önce `musteri_kayitlari.yedek.json` |
| Ayarlar | `ayarlar.json`: gönderen WhatsApp numarası, demo renkleri, bölüm anahtarları, marka cümlesi/buton metni ve köşe biçimi; açılışta geri okunur. Bu alanları içermeyen eski dosyada bölümler açık, metinler otomatik sayılır |
| Demo kaynağı | `demos/kaynak/<sayfa>.json`: sayfanın üretildiği işletme kaydı; demo yeniden üretimi buradan beslenir, bu dosya yoksa arama önbelleğine bakılır |
| Teklif | Paket seçilince varsayılan tutar yazılır (4.500 / 8.500 / 750 ₺); elle değiştirilebilir |
| Ödeme | Teklif verilince "bekliyor"; kısmi tutar ayrı tutulur, kalan bakiye hesaplanır |
| Yayın | Demo ve karne `gh-pages` dalına gider; "Yayından kaldır" dosyaları siler |
| Sektör verimi | Sol paneldeki düğme, önbellekteki taramalara göre hangi sektörün kayıt döndürdüğünü günlüğe yazar |

Gizli değerler `.env` dosyasından okunur (`GOOGLE_PLACES_API_KEY`); `.env`,
`.env` ile üretilen çalışma dosyaları ve üretilen `demos/` klasörü sürüm
kontrolüne girmez.

## Zeytin Takip — WEB Sürümü

Masaüstü uygulamasının (app.exe) web tabanlı sürümü. Aynı alan/şema mantığıyla
satıcı alımları, teslimat (satış) fişleri, giderler, kalibre fiyatları ve
komisyon takibi yapar. Flask + SQLite ile yazıldı; ek bağımlılık yok denecek
kadar azdır.

## Klasördeki dosyalar

| Dosya/Klasör        | Açıklama |
|---------------------|----------|
| `app.py`            | Flask web uygulaması (tüm ekranlar) |
| `webdb.py`          | SQLite veri katmanı (`data/zeytin_takip.db`) |
| `webpdf.py`         | Alım fişi PDF üretimi |
| `pdfgen.py`         | Sıfır bağımlılıklı PDF motoru (masaüstüyle aynı) |
| `wsgi.py`           | Yayın sunucusu giriş noktası |
| `templates/`        | Sayfa şablonları |
| `static/`           | CSS, JS, logo |
| `data/`             | Veritabanı burada oluşur (yedeklemeyi unutmayın) |

## 1) Bilgisayarda çalıştırma (deneme)

Python 3.10+ kurulu olmalı. Klasörde:

```bash
python -m venv .venv
.venv\Scripts\activate          # Windows
pip install -r requirements.txt
python app.py
```

Tarayıcıda: http://127.0.0.1:5000

İlk açılışta **İlk Kurulum** ekranından yönetici kullanıcı adı/şifre oluşturulur.
Şifre en az 8 karakter olmalı, bir harf ve bir rakam içermeli, kullanıcı adını
içermemelidir; zayıf şifreler reddedilir. Aynı kural üyelik kaydında ve panelden
şifre belirlerken de geçerlidir. Ardından bu bilgilerle giriş yapılır.

## 2) Kendi alan adınızda (domainde) yayınlama

Veritabanı ve hesap bilgileri barındırdığı için **ücretsiz/ortak** platformlar
yerine kendi sunucunuz (VPS) veya PaaS önerilir. İki yaygın yol:

### A) VPS üzerinde (Ubuntu + Nginx önerilir)

```bash
# Sunucuya kopyalayın (ör. scp -r KARAOGLU_Zeytin_Takip_Web/ kullanici@sunucu:~/karaoglu-web)
cd ~/karaoglu-web
python3 -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
# Üretim sunucusu (waitress):
waitress-serve --host=127.0.0.1 --port=8000 wsgi:app
```

Nginx site yapılandırması (`/etc/nginx/sites-available/karaoglu`):

```nginx
server {
    listen 80;
    server_name alanadiniz.com;          # ve/veya www.alanadiniz.com

    location /static/ {
        alias /home/kullanici/karaoglu-web/static/;
        expires 7d;
    }
    location / {
        proxy_pass http://127.0.0.1:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

```bash
sudo ln -s /etc/nginx/sites-available/karaoglu /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
```

Alan adınızın DNS **A** kaydını sunucu IP'nize yönlendirin.
HTTPS için (önerilir):

```bash
sudo apt install certbot python3-certbot-nginx
sudo certbot --nginx -d alanadiniz.com
```

Sunucu yeniden başlayınca otomatik çalışması için systemd servisi
(`/etc/systemd/system/karaoglu-web.service`):

```ini
[Unit]
Description=KARAOĞLU Zeytin Takip Web
After=network.target

[Service]
User=kullanici
WorkingDirectory=/home/kullanici/karaoglu-web
Environment=KARAOGLU_SECRET_KEY=cok-gizli-uzun-bir-anahtar
ExecStart=/home/kullanici/karaoglu-web/.venv/bin/waitress-serve --host=127.0.0.1 --port=8000 wsgi:app
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable --now karaoglu-web
```

### B) Render.com ile yayınlama

Depoda `render.yaml` (Render Blueprint) hazır: panelde tek tıkla tüm ayarlar
otomatik gelir.

1. Kodu GitHub'a yükleyin (`data/`, `.venv/`, `__pycache__/` hariç — `.gitignore` halleder).
2. render.com → **New + → Blueprint** → deponuzu seçin → **Apply**.
   (Blueprint yerine **Web Service** kurarsanız:
   Build: `pip install -r requirements.txt`,
   Start: `waitress-serve --host=0.0.0.0 --port=$PORT wsgi:app`,
   Health Check Path: `/saglik`)
3. Deploy bitince Render size `https://<servis-adı>.onrender.com` adresi verir.
4. Custom Domain ile alan adınızı bağlayın.

**Önemli — ücretsiz plan sınırları:**

- Site 15 dakika trafik almazsa **uykuya girer**; sonraki ilk istek ~1 dakika
  sürer ve bu sırada Render'ın "WELCOME TO RENDER / APPLICATION LOADING"
  sayfası görünür. Bu ekran 1-2 dakikadan uzun takılı kalıyorsa servis
  açılamamıştır: Render panelden **Logs** sekmesine bakın (aşağıya bakın).
- Dosya sistemi geçicidir: her yeniden başlatmada `data/` içeriği (SQLite
  veritabanı, logo, kullanıcılar) **silinir**. Verilerinizin kalıcı olması için
  ücretli planda **Persistent Disk** ekleyin (mount: `/opt/karaoglu/data`,
  `KARAOGLU_DATA_DIR=/opt/karaoglu/data`) veya VPS (A seçeneği) kullanın.

**"Application Loading" ekranında takılıyorsa (kontrol listesi):**

1. Render → Logs: `Now listening on port 10000` benzeri satır var mı? Yoksa
   start komutu hatalıdır (Procfile/render.yaml'daki komut kullanılır).
2. Start komutu `0.0.0.0` ve `$PORT` kullanmalı; `127.0.0.1` veya sabit port
   Render'da çalışmaz.
3. Health Check Path `(/saglik)` 200 dönmeli; panelden "Check /saglik" ile test edin.
4. Build loglarında `pip install -r requirements.txt` hatası varsa bağımlılıkları
   düzeltin (`flask>=3.0`, `waitress>=3.0`).
5. Ücretsiz plan instance saat limitini (750 saat/ay) doldurduysa servis ay sonuna
   kadar askıya alınır; Settings → planı `Starter` yapın.

## Ortam değişkenleri

| Değişken | Varsayılan | Ne yapar |
|---|---|---|
| `KARAOGLU_SECRET_KEY` | `data/secret_key` (otomatik üretilir) | Oturum çerezlerinin imza anahtarı; yayında mutlaka verin |
| `KARAOGLU_DATA_DIR` | `./data` | Veritabanı, logo ve yedek damgasının tutulduğu klasör |
| `KARAOGLU_GIRIS_AZAMI_HATA` | `5` (1-20) | Bu sayıda hatalı denemeden sonra geçici kilit |
| `KARAOGLU_GIRIS_KILIT_DAKIKA` | `15` (1-1440) | Kilidin süresi (dakika) |
| `PORT` | `8000` | `wsgi.py` doğrudan çalıştırıldığında dinlenecek port |

Örnek: `KARAOGLU_GIRIS_AZAMI_HATA=3 KARAOGLU_GIRIS_KILIT_DAKIKA=30` ile
3 denemede 30 dakikalık kilit uygulanır.

## Güvenlik notları

- `KARAOGLU_SECRET_KEY` ortam değişkenini mutlaka uzun/rastgele belirleyin.
- HTTPS kullanın (Let's Encrypt ücretsizdir).
- `data/` klasörünü düzenli yedekleyin (Ayarlar → Yedek İndir de mevcut).
- Şifreler PBKDF2-SHA256 ile hash'lenir; hiçbir yerde düz metin saklanmaz.
- Tüm formlar CSRF jetonuyla korunur; jeton oturuma bağlıdır ve her formda
  gizli alan olarak taşınır.
- Girişte kaba kuvvet koruması vardır: `KARAOGLU_GIRIS_AZAMI_HATA` (varsayılan
  **5**, aralık 1-20) hatalı denemeden sonra hesap + IP için
  `KARAOGLU_GIRIS_KILIT_DAKIKA` (varsayılan **15** dakika, aralık 1-1440)
  süreyle geçici kilit uygulanır. Başarılı giriş sayacı sıfırlar. Boş,
  sayı olmayan veya aralık dışı değerler varsayılana döner. Değerler açılışta
  okunur; değiştirdikten sonra servisi yeniden başlatın.

## Masaüstü verilerinizi taşımak isterseniz

Masaüstü uygulamasının veritabanı şeması birebir aynıdır:
`%APPDATA%\ZeytinTakip\zeytin_takip.db` dosyasını bu klasördeki `data/`
içine `zeytin_takip.db` adıyla kopyalamanız yeterlidir; web sürümü aynen
kullanır.
