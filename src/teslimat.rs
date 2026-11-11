//! Hizmet çıktıları: satılan her hizmetin müşteriye teslim edilebilir dosyası.
//!
//! Teklifte söz verilen işlerin bir kısmı yerelde üretilebilir: profil kontrol
//! listesi, dijital menü, citation listesi, aylık rapor, SEO planı, AI
//! görünürlük ölçüm formu, reklam metinleri, sosyal takvim ve mesaj şablonları.
//! Hepsi tek dosyalık HTML olarak `teslimat/` klasörüne yazılır; istenirse
//! mevcut GitHub Pages yayın hattıyla (`gh-pages`) herkese açık linke çevrilir.
//!
//! Sınır bilinçli: buradaki dosyalar işin *yapılabilir hâlini* verir (kontrol
//! listesi, metin, plan, ölçüm formu). Google/Ads panelinde tıklama, WhatsApp'ta
//! otomatik yanıt gibi hesap gerektiren adımlar hâlâ elle yapılır; uydurma veri
//! üretilmez.

use crate::*;
use std::fs;

/// Teslim dosyalarının yazıldığı klasör (yeniden üretilebilir, depoya girmez).
pub const KLASOR: &str = "teslimat";

/// Üretilen tek dosya.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cikti {
    pub dosya: String,
    pub baslik: String,
    pub html: String,
}

fn kacis(metin: &str) -> String {
    metin
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn kimlik(anahtar: &str) -> String {
    anahtar
        .chars()
        .map(|karakter| {
            if karakter.is_ascii_alphanumeric() || karakter == '-' {
                karakter
            } else {
                '_'
            }
        })
        .collect()
}

fn slug(ad: &str) -> String {
    let ham: String = ad
        .chars()
        .flat_map(char::to_lowercase)
        .map(|karakter| {
            if karakter.is_alphanumeric() {
                karakter
            } else {
                '-'
            }
        })
        .collect();
    let temiz: Vec<&str> = ham.split('-').filter(|parca| !parca.is_empty()).collect();
    if temiz.is_empty() {
        "isletme".to_string()
    } else {
        temiz.join("-")
    }
}

/// Müşteriye giden her sayfanın kabuğu: ad, başlık, tarih ve tek renk paleti.
fn sayfa(baslik: &str, isletme: &musteriler::Musteri, gun: &str, govde: &str) -> String {
    let ad = kacis(&isletme.ad);
    format!(
        r#"<!DOCTYPE html>
<html lang="tr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{baslik} · {ad}</title>
<style>
:root{{--z:#f7f8fa;--k:#ffffff;--y:#101828;--s:#5b6473;--c:#0f7a4d;--k2:#e4e7ec}}
*{{box-sizing:border-box}}
body{{margin:0;background:var(--z);color:var(--y);font:15px/1.6 system-ui,-apple-system,"Segoe UI",sans-serif}}
main{{max-width:820px;margin:0 auto;padding:40px 20px 64px}}
header{{border-bottom:2px solid var(--y);padding-bottom:14px;margin-bottom:26px}}
.etiket{{font-size:11px;letter-spacing:.14em;text-transform:uppercase;color:var(--c);font-weight:700}}
h1{{font-size:26px;margin:6px 0 2px}}
.alt{{margin:0;color:var(--s);font-size:13px}}
h2{{font-size:17px;margin:28px 0 10px}}
p{{margin:8px 0}}
ul,ol{{margin:8px 0 8px 18px;padding:0}}
li{{margin:4px 0}}
table{{width:100%;border-collapse:collapse;margin:10px 0 18px;background:var(--k);font-size:14px}}
th,td{{border:1px solid var(--k2);padding:8px 10px;text-align:left;vertical-align:top}}
th{{background:#eef1f5;font-size:12px;text-transform:uppercase;letter-spacing:.05em;color:var(--s)}}
.kutu{{background:var(--k);border:1px solid var(--k2);border-radius:10px;padding:14px 16px;margin:12px 0}}
.iyi{{color:var(--c);font-weight:600}}
.eksik{{color:#b42318;font-weight:600}}
small,footer{{color:var(--s);font-size:12px}}
footer{{margin-top:34px;border-top:1px solid var(--k2);padding-top:12px}}
</style></head>
<body><main>
<header><span class="etiket">Nexus Enterprise</span><h1>{ad}</h1>
<p class="alt">{baslik} · {gun}</p></header>
{govde}
<footer>Bu dosya {gun} tarihinde {ad} için hazırlandı. Fiyat ve taahhüt bilgisi teklifte yazılıdır.</footer>
</main></body></html>
"#
    )
}

fn iletisim_kutusu(isletme: &musteriler::Musteri) -> String {
    let mut satirlar = String::new();
    for (etiket, deger) in [
        ("Adres", isletme.adres.trim()),
        ("Telefon", isletme.telefon.trim()),
        ("Sektör", isletme.kategori.trim()),
        ("Web", isletme.site_linki.trim()),
    ] {
        if !deger.is_empty() {
            satirlar.push_str(&format!("<li><b>{etiket}:</b> {}</li>", kacis(deger)));
        }
    }
    if satirlar.is_empty() {
        return String::new();
    }
    format!("<div class=\"kutu\"><ul>{satirlar}</ul></div>")
}

/// Google profil kurulum/kontrol listesi: alan alan durum ve yapılacak iş.
fn profil_govdesi(isletme: &musteriler::Musteri, eksikler: &[nap::Eksik]) -> String {
    let mut satirlar = String::new();
    let alanlar = [
        (
            "İşletme adı",
            isletme.ad.trim(),
            "Şube adıyla birlikte, tabeladaki yazımla aynı olmalı.",
        ),
        (
            "Adres",
            isletme.adres.trim(),
            "Mahalle, cadde, kapı no; haritada pin aynı noktada olmalı.",
        ),
        (
            "Telefon",
            isletme.telefon.trim(),
            "Cep numarası; WhatsApp ve arama aynı hattan yürür.",
        ),
        (
            "Sektör",
            isletme.kategori.trim(),
            "En yakın birincil kategori + 3 ikincil kategori seçilmeli.",
        ),
        (
            "Web sitesi",
            isletme.site_linki.trim(),
            "Yoksa site kurulumu; varsa https ve mobil uyum kontrolü.",
        ),
        (
            "Çalışma saatleri",
            "",
            "Resmî tatil ve ramazan saatleri ayrıca girilmeli.",
        ),
        (
            "Fotoğraflar",
            "",
            "En az 10 foto: dış cephe, iç mekân, ürün, ekip.",
        ),
        (
            "Açıklama",
            "",
            "750 karakter: ne yapıyorsun, hangi semte hizmet, fark ne.",
        ),
    ];
    for (alan, deger, aksiyon) in alanlar {
        let durum = if deger.is_empty() {
            "<span class=\"eksik\">eksik</span>".to_string()
        } else {
            format!("<span class=\"iyi\">var</span> · {}", kacis(deger))
        };
        satirlar.push_str(&format!(
            "<tr><td>{}</td><td>{durum}</td><td>{}</td></tr>",
            kacis(alan),
            kacis(aksiyon)
        ));
    }
    let mut eksik_listesi = String::new();
    for eksik in eksikler {
        eksik_listesi.push_str(&format!(
            "<li>{}<b>{}</b>: {}</li>",
            if eksik.kritik { "KRİTİK · " } else { "" },
            kacis(eksik.alan),
            kacis(&eksik.mesaj)
        ));
    }
    if eksik_listesi.is_empty() {
        eksik_listesi
            .push_str("<li>Kayıtta eksik görünmüyor; profil yine de elle kontrol edilmeli.</li>");
    }
    format!(
        "<h2>Alan alan kontrol</h2><table><tr><th>Alan</th><th>Durum</th><th>Yapılacak</th></tr>{satirlar}</table>
<h2>Kayıttan gelen eksikler</h2><ul>{eksik_listesi}</ul>
<h2>Hazır profil açıklaması</h2><div class=\"kutu\">{} {}. {} bölgesinde hizmet veriyoruz. Adres: {}. Telefon: {}. Randevu ve fiyat için WhatsApp'tan yazabilirsiniz.</div>
<h2>Yapılacaklar</h2><ol>
<li>Profili işletme sahibinin Google hesabından aç (yetki olmadan değişiklik yapılmaz).</li>
<li>Ad, adres, telefonu birebir gir; haritada pini kontrol et.</li>
<li>Kategori, hizmetler, açıklama ve 10 fotoğrafı yükle.</li>
<li>Doğrulama (kartpostal/telefon/video) tamamlanana kadar bekle.</li>
<li>İlk yorumları iste; gelen yorumlara 48 saat içinde yanıt ver.</li>
</ol>",
        kacis(isletme.ad.trim()),
        kacis(isletme.kategori.trim()),
        "Bölge",
        kacis(isletme.adres.trim()),
        kacis(isletme.telefon.trim())
    )
}

/// Dijital menü / fiyat listesi: sektöre göre boş fiyat satırları.
fn menu_govdesi(isletme: &musteriler::Musteri) -> String {
    let kategori = isletme.kategori.to_lowercase();
    let yemek = [
        "restoran", "kafe", "lokanta", "döner", "pizza", "fırın", "pastane", "çiğ", "köfte",
        "burger", "kahve", "tatlı",
    ]
    .iter()
    .any(|kelime| kategori.contains(kelime));
    let bolumler: &[&str] = if yemek {
        &["Başlangıçlar", "Ana yemekler", "İçecekler", "Tatlılar"]
    } else {
        &["Hizmetler", "Fiyat listesi", "Kampanyalar", "Notlar"]
    };
    let mut govde = String::new();
    for bolum in bolumler {
        let mut satirlar = String::new();
        for _ in 0..4 {
            satirlar.push_str("<tr><td>&nbsp;</td><td>&nbsp;</td><td>___ TL</td></tr>");
        }
        govde.push_str(&format!(
            "<h2>{}</h2><table><tr><th>Ürün / hizmet</th><th>Açıklama</th><th>Fiyat</th></tr>{satirlar}</table>",
            kacis(bolum)
        ));
    }
    format!(
        "<p>Bu sayfa QR kod ile okutulur; fiyatlar işletme tarafından doldurulur. Menü değişince dosya yeniden yayınlanır, eski QR çalışmaya devam eder.</p>{}{govde}
<div class=\"kutu\"><small>QR kod, yayın linki oluştuktan sonra basılır. Aynı link kaldığı sürece masa kartlarını değiştirmek gerekmez.</small></div>",
        iletisim_kutusu(isletme)
    )
}

/// Aylık görünürlük raporu: puan, eksikler, kapsam ve ödenen bedel.
fn rapor_govdesi(
    isletme: &musteriler::Musteri,
    kayit: &gelir::MusteriGelir,
    eksikler: &[nap::Eksik],
    gun: &str,
) -> String {
    let kritik = eksikler.iter().filter(|eksik| eksik.kritik).count();
    let mut kapsam = String::new();
    for hizmet in kayit.kapsam() {
        kapsam.push_str(&format!("<li>{}</li>", kacis(hizmet)));
    }
    if kapsam.is_empty() {
        kapsam.push_str("<li>Bu ay için hizmet seçilmemiş.</li>");
    }
    let mut eksik_satirlari = String::new();
    for eksik in eksikler {
        eksik_satirlari.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            kacis(eksik.alan),
            kacis(&eksik.mesaj),
            if eksik.kritik { "kritik" } else { "uyarı" }
        ));
    }
    if eksik_satirlari.is_empty() {
        eksik_satirlari.push_str("<tr><td colspan=\"3\">Kayıtta eksik yok.</td></tr>");
    }
    format!(
        "<h2>Özet</h2>
<div class=\"kutu\"><b>Profil puanı:</b> {}/100 · <b>Kritik eksik:</b> {kritik} · <b>Toplam eksik:</b> {} · <b>Rapor günü:</b> {gun}</div>
<h2>Bu ay verilen hizmetler</h2><ul>{kapsam}</ul>
<h2>Aylık bedel</h2><p>{} TL/ay{}.</p>
<h2>Bulunan eksikler</h2><table><tr><th>Alan</th><th>Bulgu</th><th>Önem</th></tr>{eksik_satirlari}</table>
<h2>Sonraki ay</h2><ol><li>Kritik eksikler kapatılır, ardından yorum sayısı artırılır.</li><li>Profil fotoğrafları tazelenir, sezon/hizmet duyurusu girilir.</li><li>Sıralama ve arama terimleri kontrol edilip öneri listesi çıkarılır.</li></ol>",
        isletme.puan,
        eksikler.len(),
        if kayit.aylik() > 0 {
            format!("{}", kayit.aylik())
        } else {
            "0".to_string()
        },
        if kayit.tek_sefer_toplam() > 0 {
            format!(" · tek seferlik {} TL", kayit.tek_sefer_toplam())
        } else {
            String::new()
        }
    )
}

/// Citation (rehber) listesi: eksik olan alanlar ve doldurulacak rehberler.
fn citation_govdesi(isletme: &musteriler::Musteri, eksikler: &[nap::Eksik]) -> String {
    let rehberler = [
        "Google İşletme Profili",
        "Yandex Haritalar",
        "Bing Places",
        "Apple Maps",
        "Foursquare",
        "Tripadvisor",
        "Facebook Sayfa",
        "Instagram İşletme",
        "Yellow Pages Türkiye",
        "Armut / yerel dizin",
    ];
    let mut satirlar = String::new();
    for rehber in rehberler {
        satirlar.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>___</td></tr>",
            kacis(rehber),
            kacis(isletme.ad.trim()),
            kacis(isletme.telefon.trim())
        ));
    }
    let mut eksik_satirlari = String::new();
    for eksik in eksikler {
        eksik_satirlari.push_str(&format!(
            "<li><b>{}</b>: {}</li>",
            kacis(eksik.alan),
            kacis(&eksik.mesaj)
        ));
    }
    format!(
        "<p>Amaç: aynı ad, adres ve telefon her rehberde birebir aynı olsun. Farklı yazım, yerel aramada güveni düşürür.</p>
<h2>Kayıttaki eksikler</h2><ul>{eksik_satirlari}</ul>
<h2>Rehber listesi</h2><table><tr><th>Rehber</th><th>Ad</th><th>Telefon</th><th>Durum</th></tr>{satirlar}</table>
<small>Durum sütununa: kayıtlı / yok / düzeltilecek yazın.</small>"
    )
}

/// Yerel SEO planı: sayfa yapısı, citation, aylık görevler.
fn seo_govdesi(isletme: &musteriler::Musteri) -> String {
    let kategori = isletme.kategori.trim();
    format!(
        "<p>Hedef: \"{kategori}\" aramalarında bölgede ilk sayfada görünmek. Aşağıdaki işler aylık yürütülür.</p>
<h2>1. Sayfa yapısı</h2><ul>
<li>Ana sayfa: <b>{kategori}</b> + semt adı başlıkta.</li>
<li>Hizmet sayfaları: her hizmet için ayrı sayfa (ör. \"{kategori} fiyatları\").</li>
<li>Semt sayfaları: hizmet verilen her mahalle/semt için tek sayfa.</li>
<li>İletişim: adres, harita, telefon, çalışma saatleri, WhatsApp bağlantısı.</li>
</ul>
<h2>2. Teknik kontrol</h2><ul>
<li>Mobil hız: 3 saniyenin altı, görseller sıkıştırılmış.</li>
<li>Başlık (title) ve açıklama (description) her sayfada farklı.</li>
<li>Google Search Console bağlı, site haritası gönderilmiş.</li>
<li>Şema işaretleme: LocalBusiness + hizmet + SSS.</li>
</ul>
<h2>3. Aylık görevler</h2><ol>
<li>2 yeni hizmet/semt sayfası yayına alınır.</li>
<li>4 Google paylaşımı (foto + kısa metin) girilir.</li>
<li>Yorum sayısı en az 4 artırılır, hepsine yanıt verilir.</li>
<li>Sıralama ve arama terimleri kaydedilir; düşen sayfa için içerik güncellenir.</li>
</ol>
<div class=\"kutu\"><small>Not: Sıralama garantisi verilmez (Google politikası). Ölçülen şey: görünürlük, arama terimi sayısı ve gelen arama/WhatsApp.</small></div>"
    )
}

/// AI görünürlük raporu: asistanlara sorulacak sorular ve ölçüm formu.
fn geo_govdesi(isletme: &musteriler::Musteri) -> String {
    let kategori = isletme.kategori.trim();
    let semt = isletme.adres.trim();
    let sorular = [
        format!("{semt} en iyi {kategori} hangisi?"),
        format!("{semt} {kategori} fiyatları nedir?"),
        format!("{semt} {kategori} açık mı, kaça kadar?"),
        format!("{kategori} için {semt} tavsiye eder misin?"),
        format!("{} nasıl gidilir, adresi ne?", isletme.ad.trim()),
        format!("{semt} {kategori} yorumları güvenilir mi?"),
    ];
    let mut satirlar = String::new();
    for soru in &sorular {
        satirlar.push_str(&format!(
            "<tr><td>{}</td><td>___</td><td>___</td></tr>",
            kacis(soru)
        ));
    }
    format!(
        "<p>AI asistanları (ChatGPT, Gemini, Perplexity) cevap verirken profilden, siteden ve rehberlerden okuyor. Ölçüm: soru sorulur, işletmenin adı geçiyor mu bakılır.</p>
<h2>Ölçüm formu</h2><table><tr><th>Soru</th><th>Adı geçti mi?</th><th>Kaçıncı sırada?</th></tr>{satirlar}</table>
<h2>Görünürlüğü artıran işler</h2><ol>
<li>Siteye SSS bölümü: gerçek sorular + net cevaplar (fiyat aralığı, süre, konum).</li>
<li>Her hizmet için ayrı sayfa; makine okunur başlık ve şema işaretleme.</li>
<li>Rehberlerde tek tip ad-adres-telefon (citation).</li>
<li>Yorumlarda geçen hizmet adlarını sayfa metnine eklemek.</li>
<li>Aylık ölçüm: aynı sorular tekrar sorulur, değişim kaydedilir.</li>
</ol>"
    )
}

/// Google Ads metinleri: başlık/açıklama sınırlarına uygun taslaklar.
fn ads_govdesi(isletme: &musteriler::Musteri) -> String {
    let kategori = isletme.kategori.trim();
    let ad = isletme.ad.trim();
    let basliklar: Vec<String> = vec![
        format!("{ad}"),
        format!("{kategori} · {ad}"),
        format!("{kategori} Fiyatları"),
        format!("Hemen Ara: {}", isletme.telefon.trim()),
        format!("{kategori} Randevu"),
    ]
    .into_iter()
    .map(|baslik| baslik.chars().take(30).collect())
    .collect();
    let aciklamalar: Vec<String> = vec![
        format!(
            "{ad} · {kategori}. Adres: {}. Hemen arayın.",
            isletme.adres.trim()
        ),
        "Aynı gün dönüş, net fiyat. WhatsApp'tan yazın, uygun saati seçin.".to_string(),
        "Bölgede hizmet veriyoruz. Yorumlarımıza bakın, sonra karar verin.".to_string(),
        "Randevu almadan gelmeyin; yoğun saatlerde bekleme olabilir.".to_string(),
    ]
    .into_iter()
    .map(|aciklama| aciklama.chars().take(90).collect())
    .collect();
    let anahtarlar = [
        format!("{kategori}"),
        format!("{kategori} fiyat"),
        format!("{kategori} randevu"),
        format!("yakınımdaki {kategori}"),
        format!("{kategori} {}", isletme.adres.trim()),
    ];
    let mut baslik_listesi = String::new();
    for baslik in &basliklar {
        baslik_listesi.push_str(&format!("<li>{}</li>", kacis(baslik)));
    }
    let mut aciklama_listesi = String::new();
    for aciklama in &aciklamalar {
        aciklama_listesi.push_str(&format!("<li>{}</li>", kacis(aciklama)));
    }
    let mut kelime_listesi = String::new();
    for kelime in &anahtarlar {
        kelime_listesi.push_str(&format!("<li>{}</li>", kacis(kelime)));
    }
    format!(
        "<h2>Başlıklar (en fazla 30 karakter)</h2><ul>{baslik_listesi}</ul>
<h2>Açıklamalar (en fazla 90 karakter)</h2><ul>{aciklama_listesi}</ul>
<h2>Anahtar kelimeler</h2><ul>{kelime_listesi}</ul>
<h2>Kurulum adımları</h2><ol>
<li>Google Ads hesabı açılır, dönüşüm takibi (telefon + WhatsApp tıklaması) kurulur.</li>
<li>Kampanya: Arama ağı, günlük bütçe belirlenir; reklam bütçesi yönetim ücretinden ayrıdır.</li>
<li>2 reklam grubu: {kategori} ve yakınımdaki {kategori}.</li>
<li>İlk 7 gün günlük kontrol; arama terimleri raporundan negatif kelime eklenir.</li>
</ol>"
    )
}

/// Sosyal medya takvimi: dört hafta, haftada üç fikir.
fn sosyal_govdesi(isletme: &musteriler::Musteri) -> String {
    let kategori = isletme.kategori.trim();
    let haftalar = [
        (
            "1. hafta",
            [
                "İşletmeden bir kare + tek cümle fayda.".to_string(),
                format!("{kategori} hakkında en çok sorulan soruya cevap."),
                "Ekip veya ustanın kısa tanıtımı.".to_string(),
            ],
        ),
        (
            "2. hafta",
            [
                "Müşteri yorumundan alıntı (izin alınarak).".to_string(),
                "Ürün/hizmet yakın plan fotoğraf.".to_string(),
                "Çalışma saatleri ve konum hatırlatması.".to_string(),
            ],
        ),
        (
            "3. hafta",
            [
                "Öncesi/sonrası veya süreç videosu.".to_string(),
                format!("{kategori} seçerken dikkat edilecek 3 nokta."),
                "Kampanya ya da haftanın fırsatı.".to_string(),
            ],
        ),
        (
            "4. hafta",
            [
                "Sık sorulan 3 soruyu tek gönderide yanıtla.".to_string(),
                "Müşteriye teşekkür / ayın yorumu.".to_string(),
                "Gelecek ay için randevu çağrısı.".to_string(),
            ],
        ),
    ];
    let mut govde = String::new();
    for (hafta, fikirler) in haftalar {
        let mut liste = String::new();
        for fikir in fikirler {
            liste.push_str(&format!(
                "<tr><td>{}</td><td><input type=\"checkbox\"> yapıldı</td></tr>",
                kacis(&fikir)
            ));
        }
        govde.push_str(&format!(
            "<h2>{}</h2><table><tr><th>Gönderi fikri</th><th>Durum</th></tr>{liste}</table>",
            kacis(hafta)
        ));
    }
    format!("<p>Haftada 3 gönderi yeterli. Her gönderiye konum etiketi ve tek bir çağrı (ara / yaz / gel) eklenir.</p>{govde}")
}

/// Site teslim ve bakım: yayın linki, SSL, yedek, güncelleme.
fn site_govdesi(isletme: &musteriler::Musteri) -> String {
    let link = isletme.yayin_linki.as_deref().unwrap_or("").trim();
    let link_satiri = if link.is_empty() {
        "<li>Yayın linki: <span class=\"eksik\">henüz yok</span> — demo yayına gönderilince buraya yazılır.</li>".to_string()
    } else {
        format!(
            "<li>Yayın linki: <a href=\"{}\">{}</a></li>",
            kacis(link),
            kacis(link)
        )
    };
    format!(
        "<h2>Teslim</h2><ul>{link_satiri}
<li>SSL: yayın sağlayıcısı üzerinden otomatik (https).</li>
<li>Yedek: her değişiklikte sürüm geçmişine işlenir; geri dönüş tek komut.</li>
<li>Mobil uyum: 360 px genişlikte kontrol edilir.</li>
</ul>
<h2>Aylık bakım</h2><ol>
<li>Metin ve fotoğraf güncellemesi (en fazla 2 tur).</li>
<li>Kırık bağlantı ve form/WhatsApp butonu kontrolü.</li>
<li>Sayfa hızı ve mobil görünüm kontrolü.</li>
<li>Yayın linkinin açıldığının doğrulanması.</li>
</ol>
<div class=\"kutu\"><small>Site dosyaları statik olarak yayınlanır: sunucu bakımı gerekmez, erişim için hosting paneli şart değildir.</small></div>"
    )
}

/// Mesaj şablonları: WhatsApp için hazır yanıtlar.
fn mesaj_govdesi(isletme: &musteriler::Musteri) -> String {
    let ad = isletme.ad.trim();
    let tel = isletme.telefon.trim();
    let sablonlar = [
        (
            "Karşılama",
            format!("Merhaba, {ad} olarak yazıyoruz. Nasıl yardımcı olabiliriz? Fiyat, randevu veya adres için yazabilirsiniz."),
        ),
        (
            "Fiyat sorusu",
            format!("Merhaba, ilgilendiğiniz hizmetin fiyatı için kısa bir bilgi verelim. Hangi hizmet ve hangi gün düşünüyorsunuz? ({tel})"),
        ),
        (
            "Randevu",
            "Uygun saatlerimiz: hafta içi 10:00–19:00, cumartesi 10:00–17:00. Hangisi size uyar?".to_string(),
        ),
        (
            "Kapalıyız",
            "Şu anda kapalıyız. Mesajınızı bırakın, açılışta ilk sizden dönüş yapacağız.".to_string(),
        ),
        (
            "Yorum isteme",
            format!("Merhaba, {ad} olarak hizmetimizden memnun kaldıysanız Google'da kısa bir yorum bırakır mısınız? Linki gönderelim."),
        ),
        (
            "Randevu hatırlatma",
            "Yarınki randevunuzu hatırlatmak istedik. Saat değişikliği gerekirse bu mesaja yanıt verin.".to_string(),
        ),
    ];
    let mut govde = String::new();
    for (baslik, metin) in sablonlar {
        govde.push_str(&format!(
            "<h2>{}</h2><div class=\"kutu\">{}</div>",
            kacis(baslik),
            kacis(&metin)
        ));
    }
    format!("<p>Şablonlar elle gönderilir; otomatik yanıt için WhatsApp Business API gerekir. Kopyala-yapıştır ile kullanın.</p>{govde}")
}

/// Yorum toplama planı.
fn yorum_govdesi(isletme: &musteriler::Musteri) -> String {
    format!(
        "<h2>Adımlar</h2><ol>
<li>Hizmet sonunda memnun müşteriye tek mesaj: kısa yorum linki.</li>
<li>QR kod: kasada/tezgahta, \"yorum bırak\" yazısıyla.</li>
<li>Her yoruma 48 saat içinde yanıt; olumsuzda savunma değil çözüm yaz.</li>
<li>Ayda en az 4 yeni yorum hedefi.</li>
</ol>
<h2>Yorum isteme mesajı</h2><div class=\"kutu\">Merhaba, {} olarak bizi tercih ettiğiniz için teşekkürler. Google'da kısa bir yorum bırakırsanız çok seviniriz: profil sayfamızdan \"yorum yaz\" bölümü.</div>
<h2>Olumsuz yorum yanıtı</h2><div class=\"kutu\">Yaşadığınız durum için üzgünüz. Konuyu düzeltmek isteriz; {}. Uygun bir saatte bize ulaşın, çözüm için buradayız.</div>",
        kacis(isletme.ad.trim()),
        kacis(isletme.telefon.trim())
    )
}

/// Rakip / sıralama raporu: ölçüm tablosu.
fn rakip_govdesi(isletme: &musteriler::Musteri) -> String {
    let kategori = isletme.kategori.trim();
    let mut satirlar = String::new();
    for sira in 1..=5 {
        satirlar.push_str(&format!(
            "<tr><td>{sira}</td><td>&nbsp;</td><td>___</td><td>___</td><td>___</td></tr>"
        ));
    }
    format!(
        "<p>\"{kategori}\" aramasında ilk sayfadaki işletmeler doldurulur; amaç kimin hangi eksikle önde olduğunu görmek.</p>
<table><tr><th>#</th><th>İşletme</th><th>Puan</th><th>Yorum</th><th>Not</th></tr>{satirlar}</table>
<h2>Nasıl okunur</h2><ul>
<li>Puan 4.5 altındaysa yorum yönetimi öne çıkar.</li>
<li>Yorum sayısı bizden çok yüksekse hedef: ayda 4 yeni yorum.</li>
<li>Aynı semtte site yoksa site kurulumu en hızlı fark.</li>
</ul>"
    )
}

fn tur(kod: &str) -> Option<&'static str> {
    Some(match kod {
        "gbp" | "gbp_kurulum" | "gbp_kurtarma" => "profil",
        "menu_qr" => "menu",
        "yorum" => "yorum-plani",
        "nap" => "citation",
        "rapor" => "aylik-rapor",
        "kiralik_site" | "site_kurulum" | "hosting" => "site",
        "resepsiyonist" | "randevu" | "sms" => "mesaj",
        "yerel_seo" | "seo_audit" => "seo",
        "geo" | "geo_audit" => "geo",
        "rakip" => "rakip",
        "google_ads" | "ads_kurulum" => "ads",
        "sosyal" => "sosyal",
        "cekim" => "cekim",
        "egitim" => "egitim",
        _ => return None,
    })
}

fn tur_basligi(tur: &str) -> &'static str {
    match tur {
        "profil" => "Google Profil Kurulum ve Kontrol",
        "menu" => "Dijital Menü / QR",
        "yorum-plani" => "Yorum Toplama Planı",
        "citation" => "NAP ve Rehber Kayıtları",
        "aylik-rapor" => "Aylık Görünürlük Raporu",
        "site" => "Site Teslim ve Bakım",
        "mesaj" => "Hazır Mesaj Şablonları",
        "seo" => "Yerel SEO Planı",
        "geo" => "AI Görünürlük Raporu",
        "rakip" => "Rakip ve Sıralama Raporu",
        "ads" => "Google Ads Metinleri",
        "sosyal" => "Sosyal Medya Takvimi",
        "cekim" => "Çekim Planı",
        _ => "Eğitim Notları",
    }
}

fn govde(
    tur: &str,
    isletme: &musteriler::Musteri,
    kayit: &gelir::MusteriGelir,
    eksikler: &[nap::Eksik],
    gun: &str,
) -> String {
    match tur {
        "profil" => profil_govdesi(isletme, eksikler),
        "menu" => menu_govdesi(isletme),
        "yorum-plani" => yorum_govdesi(isletme),
        "citation" => citation_govdesi(isletme, eksikler),
        "aylik-rapor" => rapor_govdesi(isletme, kayit, eksikler, gun),
        "site" => site_govdesi(isletme),
        "mesaj" => mesaj_govdesi(isletme),
        "seo" => seo_govdesi(isletme),
        "geo" => geo_govdesi(isletme),
        "rakip" => rakip_govdesi(isletme),
        "ads" => ads_govdesi(isletme),
        "sosyal" => sosyal_govdesi(isletme),
        "cekim" => format!(
            "<h2>Çekim planı</h2><ol><li>Dış cephe ve tabela (sabah).</li><li>İç mekân geniş açı + detay.</li><li>Ürün/hizmet yakın çekim (en az 6 kare).</li><li>Ekip/usta portre.</li><li>Müşteriyle kısa etkileşim (izinli).</li></ol><p>Teslim: 10 düzenlenmiş fotoğraf + 2 kısa video.</p>"
        ),
        _ => format!(
            "<h2>Eğitim notları</h2><ul><li>Profil ve yorum yönetimi: haftada 15 dakika.</li><li>WhatsApp yanıt süresi: ilk 10 dakika.</li><li>Aylık rapor okuma: puan, yorum, arama terimleri.</li><li>Sık hatalar: fiyat yazmamak, yorumlara geç yanıt, eski fotoğraf.</li></ul>"
        ),
    }
}

/// Seçili hizmetlerin dosyalarını üretir (diske yazmaz).
pub fn uret(isletme: &musteriler::Musteri, kayit: &gelir::MusteriGelir, gun: &str) -> Vec<Cikti> {
    let eksikler = nap::eksikleri_topla(isletme, std::slice::from_ref(isletme));
    let mut kodlar: Vec<String> = Vec::new();
    match kayit.paket.as_deref().and_then(gelir::paket) {
        Some(paket) => kodlar.extend(paket.hizmetler.iter().map(|kod| kod.to_string())),
        None => kodlar.extend(kayit.hizmetler.iter().cloned()),
    }
    kodlar.extend(kayit.tek_seferler.iter().cloned());

    let mut ciktilar: Vec<Cikti> = Vec::new();
    for kod in &kodlar {
        let Some(tur) = tur(kod) else {
            continue;
        };
        let dosya = format!(
            "{tur}-{}-{}.html",
            slug(&isletme.ad),
            kimlik(&isletme.anahtar)
        );
        if ciktilar.iter().any(|cikti| cikti.dosya == dosya) {
            continue;
        }
        let baslik = tur_basligi(tur);
        let govde = govde(tur, isletme, kayit, &eksikler, gun);
        ciktilar.push(Cikti {
            dosya,
            baslik: baslik.to_string(),
            html: sayfa(baslik, isletme, gun, &govde),
        });
    }
    ciktilar
}

/// Dosyaları `teslimat/` klasörüne yazar.
pub fn yaz(
    isletme: &musteriler::Musteri,
    kayit: &gelir::MusteriGelir,
    gun: &str,
) -> Result<Vec<Cikti>, String> {
    yaz_klasore(isletme, kayit, gun, std::path::Path::new(KLASOR))
}

/// Test edilebilir sürüm: dosyaları verilen klasöre yazar.
pub fn yaz_klasore(
    isletme: &musteriler::Musteri,
    kayit: &gelir::MusteriGelir,
    gun: &str,
    klasor: &std::path::Path,
) -> Result<Vec<Cikti>, String> {
    let ciktilar = uret(isletme, kayit, gun);
    if ciktilar.is_empty() {
        return Err("Seçili hizmet yok: önce paket ya da hizmet işaretleyin.".to_string());
    }
    fs::create_dir_all(klasor)
        .map_err(|hata| format!("{} oluşturulamadı: {hata}", klasor.display()))?;
    for cikti in &ciktilar {
        let yol = klasor.join(&cikti.dosya);
        fs::write(&yol, &cikti.html)
            .map_err(|hata| format!("{} yazılamadı: {hata}", yol.display()))?;
    }
    Ok(ciktilar)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn isletme() -> musteriler::Musteri {
        musteriler::Musteri {
            anahtar: "osm:node/1".to_string(),
            ad: "Kantin & Kafe".to_string(),
            telefon: "0532 123 45 67".to_string(),
            adres: "Atatürk Cad. No:5".to_string(),
            kategori: "Restoran".to_string(),
            puan: 45,
            ..Default::default()
        }
    }

    fn kayit(paket: &str) -> gelir::MusteriGelir {
        gelir::MusteriGelir {
            paket: Some(paket.to_string()),
            paket_fiyati: 5_000,
            ..Default::default()
        }
    }

    #[test]
    fn buyume_paketi_bes_dosya_uretir() {
        let ciktilar = uret(&isletme(), &kayit("buyume"), "2026-11-10");
        let adlar: Vec<&str> = ciktilar.iter().map(|cikti| cikti.dosya.as_str()).collect();
        assert_eq!(ciktilar.len(), 5, "{adlar:?}");
        assert!(adlar.iter().any(|ad| ad.starts_with("profil-")));
        assert!(adlar.iter().any(|ad| ad.starts_with("aylik-rapor-")));
        assert!(adlar.iter().any(|ad| ad.starts_with("site-")));
        assert!(adlar.iter().any(|ad| ad.starts_with("citation-")));
        assert!(adlar.iter().any(|ad| ad.starts_with("yorum-plani-")));
    }

    #[test]
    fn tek_hizmet_tek_dosya_verir() {
        let kayit = gelir::MusteriGelir {
            hizmetler: vec!["google_ads".to_string(), "sosyal".to_string()],
            ..Default::default()
        };
        let ciktilar = uret(&isletme(), &kayit, "2026-11-10");
        assert_eq!(ciktilar.len(), 2);
        assert!(ciktilar.iter().any(|cikti| cikti.dosya.starts_with("ads-")));
        assert!(ciktilar
            .iter()
            .any(|cikti| cikti.dosya.starts_with("sosyal-")));
    }

    #[test]
    fn reklam_basliklari_sinira_uyar() {
        let kayit = gelir::MusteriGelir {
            hizmetler: vec!["google_ads".to_string()],
            ..Default::default()
        };
        let ciktilar = uret(&isletme(), &kayit, "2026-11-10");
        let html = &ciktilar[0].html;
        assert!(html.contains("Google Ads Metinleri"));
        assert!(html.contains("Başlıklar (en fazla 30 karakter)"));
        // En uzun başlık 30 karakteri aşmamalı.
        let baslik_bolumu = html.split("<h2>Başlıklar").nth(1).expect("başlık var");
        let liste = baslik_bolumu.split("</ul>").next().expect("liste var");
        for satir in liste.split("<li>").skip(1) {
            let metin = satir.split("</li>").next().unwrap_or("");
            assert!(metin.chars().count() <= 30, "uzun başlık: {metin}");
        }
    }

    #[test]
    fn sayfa_kacis_ve_ad_icerir() {
        let mut isletme = isletme();
        isletme.ad = "Ali & Oğlu <Kafe>".to_string();
        let ciktilar = uret(&isletme, &kayit("baslangic"), "2026-11-10");
        let html = &ciktilar[0].html;
        assert!(html.contains("Ali &amp; Oğlu &lt;Kafe&gt;"));
        assert!(html.contains("2026-11-10"));
    }

    #[test]
    fn dosyalar_diske_yazilir() {
        let klasor = std::env::temp_dir().join("nexus_teslimat_testi");
        let _ = std::fs::remove_dir_all(&klasor);
        let ciktilar = yaz_klasore(&isletme(), &kayit("buyume"), "2026-11-10", &klasor)
            .expect("dosyalar yazıldı");
        for cikti in &ciktilar {
            let yol = klasor.join(&cikti.dosya);
            assert!(yol.is_file(), "{} yok", yol.display());
            let icerik = std::fs::read_to_string(&yol).expect("dosya okundu");
            assert!(icerik.contains("Nexus Enterprise"));
            assert!(icerik.contains("Kantin &amp; Kafe") || icerik.contains("Kantin & Kafe"));
        }
        let _ = std::fs::remove_dir_all(&klasor);
    }

    #[test]
    fn hizmet_yoksa_uretim_bos_doner() {
        assert!(uret(&isletme(), &gelir::MusteriGelir::default(), "2026-11-10").is_empty());
    }
}
