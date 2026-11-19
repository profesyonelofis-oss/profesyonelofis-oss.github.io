# Pin denetimi · Saruhanlı Oto Kurtarma

- Tarih: 2026-11-10
- Kaynak: paylaşılan Google işletme profili bağlantısı (`share.google/8voLli1gBoGcAO1E0`)
- Telefon: 0507 129 94 39

## 1. Profilde bugün görünen

| Alan | Görünen değer |
|---|---|
| Ad | Saruhanlı Oto Kurtarma |
| Puan | 5,0 · 16 yorum |
| Birincil kategori | Çekici Hizmeti |
| Telefon | +90 507 129 94 39 |
| Çalışma saatleri | 24 saat açık |
| Web sitesi | `https://wa.me/905071299439` (site değil, WhatsApp) |
| Adres | Görünmüyor (hizmet alanı işletmesi) |
| Gönderi | Var (4 Eki 2026) |

## 2. Eksikler

`TAMAM` = bu depoda yapıldı · `SEN` = yalnızca profil sahibinin hesabından yapılabilir

| # | Eksik | Neden önemli | Durum |
|---|---|---|---|
| 1 | Web sitesi alanı WhatsApp'a gidiyor | Google "site" alanını güven sinyali sayar; wa.me bağlantısı site yerine geçmez | SEN (alan adı yayına alınınca) |
| 2 | Sokak adresi yok, hizmet alanı tanımlı değil | Pin, hizmet verilen 7 ilçeyle eşleşmiyor; "yakınımdaki çekici" aramasında mesafe sinyali zayıf | SEN |
| 3 | İkincil kategori yok | Tek kategori alaka alanını daraltıyor | SEN |
| 4 | Hizmet listesi boş | Hizmet adları aramada doğrudan eşleşir | SEN |
| 5 | Öznitelik yok (7/24, yerinde hizmet, kart) | Filtrelerde görünmüyorsun | SEN |
| 6 | Fotoğraf/video yok | Fotoğrafsız profilin tıklanma oranı belirgin düşük | SEN (10 gerçek kare) |
| 7 | Ürün/hizmet kartı ve fiyat notu yok | "telefonla fiyat" bilgisi profilde durmalı | SEN |
| 8 | Soru-cevap yok | Üç hazır soru aşağıda | SEN |
| 9 | `sameAs` yok (Yandex, Facebook, Instagram) | Aynı işletmenin diğer kayıtlarıyla bağ kurulmuyor | SEN |
| 10 | Site şemasında adres tipi, posta kodu, para birimi, ödeme, hizmet tipi, harita ve `sameAs` eksikti | Zengin sonuç ve NAP tutarlılığı | **TAMAM** (`site/index.html`, `site/sahneler.html`) |
| 11 | 7 ilçe sayfasının tamamı adresi hedef ilçe gösteriyordu (ör. Tuzla) | Aynı işletme 30 farklı adreste görünüyor → pin çelişkisi | **TAMAM** (30 sayfa + `src/bolgesel.rs`) |
| 12 | Sitede harita ve yorum bağlantısı yoktu | Pin ↔ site ↔ yorum döngüsü kopuktu | **TAMAM** (iletişim bölümü) |
| 13 | Kayıtlı pin koordinatı Saruhanlı merkezle uyuşmuyordu | `pinler.json`: 39.004814, 28.116757 · Saruhanlı merkez: 38.7344, 27.5681 → yaklaşık **56 km** kuzeydoğu. İşletme sahibi bu noktayı **doğruladı** (10.11.2026) | **TAMAM** (koordinat onaylandı, şema + 33 sayfa bu noktaya çekildi) |

> 13. madde kapatıldı: işletme sahibi 39.004814, 28.116757 noktasını doğru kabul etti. Nokta artık `pinler.json`, ana sayfa ve 31 bölge sayfasında aynı; `geo` ve `hasMap` alanları bu koordinatı taşıyor. Saruhanlı merkezle aradaki mesafe gerçek bir konum farkı; ilçe merkezi varsayımıyla hareket etme.

## 3. NAP standardı (her yerde birebir aynı)

```
Saruhanlı Oto Kurtarma
Saruhanlı · 45800 Manisa
+90 507 129 94 39
07:00–07:00 (24 saat)
```

Yazım varyantı kullanma: "Saruhanli", "Oto kurtarma", "0507 129 94 39" gibi farklı biçimler NAP eşleşmesini zayıflatır. Sitede görünen telefon `0507 129 94 39`, şemada `+905071299439` — ikisi aynı numaradır, Google bunu normalleştirir.

## 4. Profil açıklaması (kopyala-yapıştır · 750 karakter sınırı)

> Saruhanlı / Manisa merkezli ekibimiz O-5 otoyolunun Kemalpaşa–Savaştepe kesiminde (7 ilçe) 7/24 çekici ve oto kurtarma hizmeti veriyor. Aracınız yolda kaldıysa en yakın çekici olarak yanınızdayız: yol yardım 7/24, benzin destek, mazot takviye, akü takviye, akücü, lastik değişimi ve kaza kurtarma. Hasarsız yükleme, istediğiniz servise teslim; fiyat yola çıkmadan söylenir, teslimde değişmez. Konumunuzu WhatsApp'tan gönderin, en yakın ekibi yönlendirelim. Telefon: 0507 129 94 39

## 5. Hizmet satırları (her biri ayrı hizmet, fiyat: telefonla)

çekici · oto kurtarma · yol yardım 7/24 · en yakın çekici · benzin destek · mazot takviye · akü takviye · akücü · lastik değişimi · kaza kurtarma

## 6. İkincil kategoriler

- Oto kurtarma hizmeti
- Yol yardım hizmeti
- Akü hizmeti
- Lastikçi

Birincil kategori **Çekici Hizmeti** olarak kalır.

## 7. Öznitelikler

- Yerinde hizmet
- 7/24 hizmet
- Kredi kartı kabul edilir (gerçekse)
- Otoyol kurtarma
- Türkçe hizmet

## 8. Soru-cevap (profilden "Soru sor" ile ekle, sonra kendin yanıtla)

1. **Ne kadar sürede geliyorsunuz?** — Bölge içinde ortalama 20-30 dakika; O-5 hattında konuma göre değişir, telefonda net süre söylenir.
2. **Fiyat ne kadar?** — Mesafe ve araç durumuna göre değişir; fiyat yola çıkmadan telefonda söylenir, teslimde değişmez.
3. **Gece de geliyor musunuz?** — Evet, 7/24 çalışıyoruz; gece, hafta sonu ve resmî tatil aynı numara.

## 9. Fotoğraf sırası (10 kare)

1. Çekici yan görünüm (kapak) · 2. Araca yükleme anı · 3. Kayış/gergi detayı · 4. Gece çalışması · 5. Akü takviye · 6. Yakıt bidonu · 7. Ekip · 8. Tabela/kaplama · 9. O-5'te kurtarma · 10. Müşteriye teslim

## 10. Sıradaki beş adım

1. Profildeki konumu doğrula (madde 13) — yanlışsa düzelt, bana söyle.
2. Hizmet alanına 7 ilçeyi ekle (`plan.md` listesi).
3. Kategori + hizmet + öznitelik + açıklama (madde 4-7).
4. 10 fotoğrafı yükle.
5. Siteyi alan adına al, profildeki "Web sitesi" alanını wa.me yerine o adresle değiştir.
