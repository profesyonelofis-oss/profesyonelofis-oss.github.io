//! Nexus SEO sunucusu — %100 Rust, sunucu tarafında derlenen (SSR) HTML.
//!
//! Masaüstü paneli (`ollama_proje.exe`) yalnız medya/AI/veri üretir; site
//! üretimi ve sunumu bu bağımsız ikilidedir. Panel onayladığı her işletmeyi
//! `web/veri.json` dosyasına yazar, bu sunucu dosyayı her istekte okuyup
//! anında yayına alır (ayrı veritabanı, JS veya şablon motoru gerekmez).
//!
//! Çalıştırma:
//!   cargo run --release --bin sunucu -- [port] [kok_dizin]
//!
//! Rotalar:
//!   `/`                     → işletme dizini (SSR, SEO başlıklı)
//!   `/s/<slug>`             → landing page (SSR + JSON-LD + OG)
//!   `/robots.txt`           → sitemap bağlantılı robots
//!   `/sitemap.xml`          → otomatik sitemap
//!   `/medya/<göreli_yol>`   → `media/stock/` altındaki gerçek dosya
//!   `/saglik`               → JSON sağlık yanıtı

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Varsayılan port (köprü 8777 ve onay paneli 8080 ile çakışmaz).
pub const PORT: u16 = 8090;

/// `web/veri.json` içindeki tek medya kaydı.
#[derive(Clone, Debug, Default)]
pub struct Medya {
    pub yol: String,
    /// `foto` veya `video` (büyük/küçük harf duyarsız).
    pub tur: String,
    /// Video için kapak görseli (varsa).
    pub kapak: String,
}

/// Sunucunun yayınladığı tek işletme sayfası.
#[derive(Clone, Debug, Default)]
pub struct Site {
    pub slug: String,
    pub ad: String,
    pub sektor: String,
    pub sektor_slug: String,
    pub il: String,
    pub ilce: String,
    pub telefon: String,
    pub adres: String,
    pub aciklama: String,
    /// Motorun ürettiği hazır `<title>` (≤60). Boşsa sunucu kendi üretir.
    pub baslik_oncelik: String,
    /// Motorun seçtiği Schema.org tipi. Boşsa sektörden türetilir.
    pub sema: String,
    pub medya: Vec<Medya>,
}

/// `web/veri.json` kökü.
#[derive(Clone, Debug, Default)]
pub struct Veri {
    pub surum: u32,
    pub guncelleme: String,
    pub siteler: Vec<Site>,
}

impl Veri {
    /// JSON metnini ayrıştırır. Bozuk dosya sunucuyu düşürmez: boş veri döner.
    pub fn json(metin: &str) -> Veri {
        let Ok(deger) = serde_json::from_str::<serde_json::Value>(metin) else {
            return Veri::default();
        };
        let mut siteler = Vec::new();
        for site in deger
            .get("siteler")
            .and_then(|s| s.as_array())
            .map(|s| s.as_slice())
            .unwrap_or_default()
        {
            let metin = |alan: &str| {
                site.get(alan)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string()
            };
            let slug = metin("slug");
            if slug.trim().is_empty() {
                continue;
            }
            let mut medya = Vec::new();
            for kayit in site
                .get("medya")
                .and_then(|m| m.as_array())
                .map(|m| m.as_slice())
                .unwrap_or_default()
            {
                let yol = kayit
                    .get("yol")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .replace('\\', "/");
                if yol.trim().is_empty() {
                    continue;
                }
                medya.push(Medya {
                    yol,
                    tur: kayit
                        .get("tur")
                        .and_then(|v| v.as_str())
                        .unwrap_or("foto")
                        .to_lowercase(),
                    kapak: kayit
                        .get("kapak")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                });
            }
            siteler.push(Site {
                slug,
                ad: metin("ad"),
                sektor: metin("sektor"),
                sektor_slug: metin("sektor_slug"),
                il: metin("il"),
                ilce: metin("ilce"),
                telefon: metin("telefon"),
                adres: metin("adres"),
                aciklama: metin("aciklama"),
                baslik_oncelik: metin("baslik"),
                sema: metin("sema"),
                medya,
            });
        }
        Veri {
            surum: deger.get("surum").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
            guncelleme: deger
                .get("guncelleme")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            siteler,
        }
    }
}

/// HTML kaçışı: metin alanları etikete gömülürken `<`, `&`, `"`, `'` kaçılır.
pub fn kacis(metin: &str) -> String {
    let mut cikti = String::with_capacity(metin.len() + 16);
    for karakter in metin.chars() {
        match karakter {
            '&' => cikti.push_str("&amp;"),
            '<' => cikti.push_str("&lt;"),
            '>' => cikti.push_str("&gt;"),
            '"' => cikti.push_str("&quot;"),
            '\'' => cikti.push_str("&#39;"),
            _ => cikti.push(karakter),
        }
    }
    cikti
}

/// Sektöre göre Schema.org tipi (JSON-LD `@type`).
pub fn sema_tipi(sektor_slug: &str) -> &'static str {
    match sektor_slug.to_lowercase().as_str() {
        "restoran" | "kebap" | "kafe" | "lokanta" | "pastane" => "Restaurant",
        "cekici" | "oto" | "tamirci" | "auto" | "servis" => "AutoRepair",
        "kuafor" | "berber" | "guzellik" => "HairSalon",
        "market" | "bakkal" => "Store",
        "otel" | "pansiyon" => "LodgingBusiness",
        "emlak" => "RealEstateAgent",
        "dis" | "klinik" | "doktor" => "MedicalBusiness",
        _ => "LocalBusiness",
    }
}

/// Arama motoru sınırına sığdırır: taşarsa `…` ile keser.
fn kirp(metin: &str, sinir: usize) -> String {
    if metin.chars().count() <= sinir {
        return metin.to_string();
    }
    let mut kisa: String = metin.chars().take(sinir.saturating_sub(1)).collect();
    kisa.push('…');
    kisa
}

/// SEO başlığı: en fazla 60 karakter (SERP'te kesilmesin).
///
/// Motor hazır başlık yazdıysa o kullanılır; yoksa "ad · yer | sektör"
/// biçiminde üretilip sığana kadar sadeleştirilir.
pub fn baslik(site: &Site) -> String {
    if !site.baslik_oncelik.trim().is_empty() {
        return kirp(site.baslik_oncelik.trim(), 60);
    }
    let ad = if site.ad.trim().is_empty() {
        "İşletme"
    } else {
        site.ad.trim()
    };
    let yer = [site.ilce.as_str(), site.il.as_str()]
        .into_iter()
        .filter(|parca| !parca.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let sektor = site.sektor.trim().to_lowercase();
    let adaylar = [
        format!("{ad} · {yer} | {sektor}"),
        format!("{ad} · {yer}"),
        format!("{ad} | {sektor}"),
        ad.to_string(),
    ];
    for aday in adaylar {
        if aday.chars().count() <= 60 {
            return aday;
        }
    }
    kirp(ad, 60)
}

/// Meta description: açıklama yoksa sektör/şehirden üretilir (155 karakter).
pub fn aciklama(site: &Site) -> String {
    let kaynak = if site.aciklama.trim().is_empty() {
        let sektor = if site.sektor.trim().is_empty() {
            "hizmet"
        } else {
            site.sektor.trim()
        };
        let yer = [site.ilce.as_str(), site.il.as_str()]
            .into_iter()
            .filter(|parca| !parca.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "{sektor} · {} · {yer} adresinde hizmet. Telefon, konum ve fotoğraflar bu sayfada.",
            site.ad.trim()
        )
    } else {
        site.aciklama.trim().to_string()
    };
    kirp(kaynak.trim(), 160)
}

/// Sayfanın paylaşım görseli: önce video kapağı, sonra ilk fotoğraf.
pub fn paylasim_gorseli(site: &Site) -> Option<&str> {
    site.medya
        .iter()
        .find(|medya| !medya.kapak.trim().is_empty())
        .map(|medya| medya.kapak.as_str())
        .or_else(|| {
            site.medya
                .iter()
                .find(|medya| medya.tur != "video")
                .map(|medya| medya.yol.as_str())
        })
}

/// İlk video (OG video ve arka plan videosu için).
pub fn ilk_video(site: &Site) -> Option<&Medya> {
    site.medya.iter().find(|medya| medya.tur == "video")
}

/// Site URL'si (canonical + OG için ortamdan okunur).
pub fn kok_adres() -> String {
    std::env::var("NEXUS_SITE_ADRES").unwrap_or_else(|_| format!("http://127.0.0.1:{PORT}"))
}

/// `/medya/<yol>` isteklerini `media/stock/` altına güvenle çözer.
/// `..`, mutlak yol ve kaçış denemeleri reddedilir.
pub fn medya_yolu(kok: &Path, istek_yolu: &str) -> Option<PathBuf> {
    let goreli = istek_yolu
        .trim_start_matches('/')
        .strip_prefix("medya/")?
        .replace('\\', "/");
    if goreli.is_empty() || goreli.contains("..") || goreli.starts_with('/') || goreli.contains(':')
    {
        return None;
    }
    let aday = kok.join("media/stock").join(&goreli);
    let kanonik = aday.canonicalize().ok()?;
    let taban = kok.join("media/stock").canonicalize().ok()?;
    kanonik.starts_with(&taban).then_some(kanonik)
}

/// Uzantıya göre `content-type`.
pub fn icerik_turu(yol: &Path) -> &'static str {
    match yol
        .extension()
        .map(|uzanti| uzanti.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    }
}

/// Landing page HTML'i (tam SSR: JS gerekmez, tüm etiketler sunucuda).
pub fn sayfa(site: &Site, kok_adres: &str) -> String {
    let baslik = baslik(site);
    let aciklama = aciklama(site);
    let url = format!("{}/s/{}", kok_adres.trim_end_matches('/'), site.slug);
    let gorsel = paylasim_gorseli(site)
        .map(|yol| format!("{}/{}", kok_adres.trim_end_matches('/'), yol))
        .unwrap_or_default();
    let video = ilk_video(site);
    let sema = if site.sema.trim().is_empty() {
        sema_tipi(&site.sektor_slug)
    } else {
        site.sema.trim()
    };

    let mut jsonld = serde_json::json!({
        "@context": "https://schema.org",
        "@type": sema,
        "name": site.ad,
        "description": aciklama,
        "url": url,
        "areaServed": [site.il.clone(), site.ilce.clone()],
    });
    if !site.telefon.is_empty() {
        jsonld["telephone"] = serde_json::Value::String(site.telefon.clone());
    }
    if !site.adres.is_empty() {
        jsonld["address"] = serde_json::json!({
            "@type": "PostalAddress",
            "streetAddress": site.adres,
            "addressLocality": site.ilce,
            "addressRegion": site.il,
            "addressCountry": "TR",
        });
    }
    if !gorsel.is_empty() {
        jsonld["image"] = serde_json::Value::String(gorsel.clone());
    }
    let jsonld = serde_json::to_string(&jsonld).unwrap_or_else(|_| "{}".to_string());

    let mut medya_html = String::new();
    for kayit in &site.medya {
        let src = format!("{}/{}", kok_adres.trim_end_matches('/'), kayit.yol);
        if kayit.tur == "video" {
            let kapak = if kayit.kapak.trim().is_empty() {
                String::new()
            } else {
                format!(
                    " poster=\"{}\"",
                    kacis(&format!(
                        "{}/{}",
                        kok_adres.trim_end_matches('/'),
                        kayit.kapak
                    ))
                )
            };
            medya_html.push_str(&format!(
                "<figure class=\"medya video\"><video autoplay muted loop playsinline preload=\"metadata\"{kapak}><source src=\"{}\" type=\"{}\"></video></figure>\n",
                kacis(&src),
                icerik_turu(Path::new(&kayit.yol))
            ));
        } else {
            medya_html.push_str(&format!(
                "<figure class=\"medya\"><img src=\"{}\" alt=\"{} fotoğraf\" loading=\"lazy\" decoding=\"async\" width=\"768\" height=\"512\"></figure>\n",
                kacis(&src),
                kacis(&site.ad)
            ));
        }
    }
    if medya_html.is_empty() {
        medya_html.push_str("<p class=\"bos\">Bu işletme için henüz medya yayınlanmadı.</p>\n");
    }

    let og_video = video
        .map(|kayit| {
            format!(
                "<meta property=\"og:video\" content=\"{}\">\n<meta property=\"og:video:type\" content=\"{}\">\n",
                kacis(&format!("{}/{}", kok_adres.trim_end_matches('/'), kayit.yol)),
                icerik_turu(Path::new(&kayit.yol))
            )
        })
        .unwrap_or_default();
    let og_gorsel = if gorsel.is_empty() {
        String::new()
    } else {
        format!(
            "<meta property=\"og:image\" content=\"{}\">\n<meta name=\"twitter:image\" content=\"{}\">\n",
            kacis(&gorsel),
            kacis(&gorsel)
        )
    };
    let telefon = if site.telefon.trim().is_empty() {
        String::new()
    } else {
        format!(
            "<a class=\"telefon\" href=\"tel:{}\">{}</a>",
            kacis(&site.telefon.replace(' ', "")),
            kacis(&site.telefon)
        )
    };
    let yer = [site.ilce.as_str(), site.il.as_str()]
        .into_iter()
        .filter(|parca| !parca.trim().is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"<!DOCTYPE html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{baslik}</title>
<meta name="description" content="{aciklama}">
<meta name="robots" content="index, follow, max-image-preview:large">
<link rel="canonical" href="{url}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="Nexus Enterprise">
<meta property="og:locale" content="tr_TR">
<meta property="og:title" content="{baslik}">
<meta property="og:description" content="{aciklama}">
<meta property="og:url" content="{url}">
{og_gorsel}{og_video}<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="{baslik}">
<meta name="twitter:description" content="{aciklama}">
<script type="application/ld+json">{jsonld}</script>
<style>
:root{{--zemin:#0d1117;--kart:#161b22;--yazi:#e6edf3;--soluk:#9aa4b2;--vurgu:#2ea043}}
*{{box-sizing:border-box}}
body{{margin:0;background:var(--zemin);color:var(--yazi);font:16px/1.6 system-ui,Segoe UI,Roboto,sans-serif}}
header,main,footer{{max-width:1080px;margin:0 auto;padding:24px}}
h1{{font-size:clamp(24px,4vw,40px);margin:0 0 8px}}
.ust{{display:flex;gap:12px;flex-wrap:wrap;color:var(--soluk);font-size:14px}}
.medya{{margin:0;border-radius:14px;overflow:hidden;background:var(--kart)}}
.medya img,.medya video{{display:block;width:100%;height:auto}}
.galeri{{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:14px;margin:24px 0}}
.telefon{{display:inline-block;margin-top:12px;padding:12px 20px;border-radius:10px;background:var(--vurgu);color:#fff;text-decoration:none;font-weight:600}}
footer{{color:var(--soluk);font-size:13px;border-top:1px solid #21262d;margin-top:32px}}
a{{color:inherit}}
</style>
</head>
<body>
<header>
<h1>{ad}</h1>
<div class="ust"><span>{sektor}</span><span>{yer}</span>{telefon}</div>
</header>
<main>
<p>{aciklama_metni}</p>
<div class="galeri">
{medya_html}</div>
{adres}
</main>
<footer>Bu sayfa Nexus Enterprise tarafından sunucu tarafında üretildi · <a href="/">tüm işletmeler</a></footer>
</body>
</html>"#,
        baslik = kacis(&baslik),
        sektor = kacis(&site.sektor),
        aciklama = kacis(&aciklama),
        url = kacis(&url),
        og_gorsel = og_gorsel,
        og_video = og_video,
        jsonld = jsonld.replace("</", "<\\/"),
        ad = kacis(&site.ad),
        yer = kacis(&yer),
        telefon = telefon,
        aciklama_metni = kacis(&aciklama),
        medya_html = medya_html,
        adres = if site.adres.trim().is_empty() {
            String::new()
        } else {
            format!("<p class=\"adres\">{}</p>", kacis(&site.adres))
        },
    )
}

/// İşletme dizini (SSR).
pub fn dizin(veri: &Veri, kok_adres: &str) -> String {
    let mut kartlar = String::new();
    for site in &veri.siteler {
        kartlar.push_str(&format!(
            "<li><a href=\"/s/{}\"><strong>{}</strong><span>{} · {}</span></a></li>\n",
            kacis(&site.slug),
            kacis(&baslik(site)),
            kacis(&site.sektor),
            kacis(
                &[site.ilce.as_str(), site.il.as_str()]
                    .into_iter()
                    .filter(|parca| !parca.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        ));
    }
    if kartlar.is_empty() {
        kartlar.push_str("<li class=\"bos\">Henüz yayınlanmış işletme yok.</li>\n");
    }
    format!(
        r#"<!DOCTYPE html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>İşletme dizini | Nexus Enterprise</title>
<meta name="description" content="Manisa ve çevresindeki işletmelerin fotoğraf ve video galerileri; sektör ve ilçeye göre listelenir.">
<meta name="robots" content="index, follow">
<link rel="canonical" href="{kok}/">
<meta property="og:type" content="website">
<meta property="og:title" content="İşletme dizini | Nexus Enterprise">
<meta property="og:description" content="Sektör ve ilçeye göre işletme galerileri.">
<style>
body{{margin:0;background:#0d1117;color:#e6edf3;font:16px/1.6 system-ui,Segoe UI,sans-serif}}
main{{max-width:960px;margin:0 auto;padding:32px 24px}}
ul{{list-style:none;padding:0;display:grid;gap:12px}}
li a{{display:flex;flex-direction:column;padding:16px 18px;border-radius:12px;background:#161b22;text-decoration:none}}
li span{{color:#9aa4b2;font-size:14px}}
</style>
</head>
<body><main>
<h1>İşletme dizini</h1>
<p>Sunucu tarafında üretilen sayfalar; JavaScript gerekmez.</p>
<ul>
{kartlar}</ul>
</main></body>
</html>"#,
        kok = kacis(kok_adres.trim_end_matches('/')),
        kartlar = kartlar
    )
}

/// `robots.txt`: tüm arama motorlarına açık, sitemap bağlantılı.
pub fn robots(kok_adres: &str) -> String {
    format!(
        "User-agent: *\nAllow: /\nDisallow: /saglik\n\nSitemap: {}/sitemap.xml\n",
        kok_adres.trim_end_matches('/')
    )
}

/// `sitemap.xml`: her işletme sayfası + dizin.
pub fn sitemap(veri: &Veri, kok_adres: &str) -> String {
    let taban = kok_adres.trim_end_matches('/');
    let son = if veri.guncelleme.trim().is_empty() {
        String::new()
    } else {
        format!("<lastmod>{}</lastmod>", kacis(&veri.guncelleme))
    };
    let mut url_ler =
        format!("<url><loc>{taban}/</loc>{son}<changefreq>daily</changefreq></url>\n");
    for site in &veri.siteler {
        url_ler.push_str(&format!(
            "<url><loc>{taban}/s/{}</loc>{son}<changefreq>weekly</changefreq><priority>0.9</priority></url>\n",
            kacis(&site.slug)
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{url_ler}</urlset>\n"
    )
}

/// Tek HTTP yanıtı.
struct Yanit {
    kod: u16,
    tur: String,
    govde: Vec<u8>,
    ek_basliklar: Vec<(String, String)>,
}

impl Yanit {
    fn html(govde: String) -> Yanit {
        Yanit {
            kod: 200,
            tur: "text/html; charset=utf-8".to_string(),
            govde: govde.into_bytes(),
            ek_basliklar: vec![("Cache-Control".to_string(), "no-store".to_string())],
        }
    }

    fn metin(tur: &str, govde: String) -> Yanit {
        Yanit {
            kod: 200,
            tur: tur.to_string(),
            govde: govde.into_bytes(),
            ek_basliklar: vec![(
                "Cache-Control".to_string(),
                "public, max-age=60".to_string(),
            )],
        }
    }

    fn hata(kod: u16, mesaj: &str) -> Yanit {
        Yanit {
            kod,
            tur: "text/plain; charset=utf-8".to_string(),
            govde: mesaj.as_bytes().to_vec(),
            ek_basliklar: Vec::new(),
        }
    }
}

/// İsteği rotaya çevirir (saf fonksiyon: testler doğrudan çağırır).
pub fn yonlendir(kok: &Path, istek_yolu: &str) -> Yanit {
    let veri = veri_oku(kok);
    let adres = kok_adres();
    let yol = istek_yolu.split('?').next().unwrap_or("/");
    match yol {
        "/" | "/index.html" => Yanit::html(dizin(&veri, &adres)),
        "/robots.txt" => Yanit::metin("text/plain; charset=utf-8", robots(&adres)),
        "/sitemap.xml" => Yanit::metin("application/xml; charset=utf-8", sitemap(&veri, &adres)),
        "/saglik" => {
            let site_sayisi = veri.siteler.len();
            let surum = veri.surum;
            Yanit::metin(
                "application/json; charset=utf-8",
                format!("{{\"ok\":true,\"site\":{site_sayisi},\"surum\":{surum}}}"),
            )
        }
        _ => {
            if let Some(slug) = yol.strip_prefix("/s/") {
                let slug = slug.trim_end_matches('/');
                return match veri.siteler.iter().find(|site| site.slug == slug) {
                    Some(site) => Yanit::html(sayfa(site, &adres)),
                    None => Yanit::hata(404, "sayfa bulunamadı"),
                };
            }
            if yol.starts_with("/medya/") {
                let Some(dosya) = medya_yolu(kok, yol) else {
                    return Yanit::hata(404, "medya bulunamadı");
                };
                return match std::fs::read(&dosya) {
                    Ok(govde) => Yanit {
                        kod: 200,
                        tur: icerik_turu(&dosya).to_string(),
                        govde,
                        ek_basliklar: vec![(
                            "Cache-Control".to_string(),
                            "public, max-age=86400".to_string(),
                        )],
                    },
                    Err(_) => Yanit::hata(404, "medya okunamadı"),
                };
            }
            Yanit::hata(404, "sayfa bulunamadı")
        }
    }
}

/// `web/veri.json` okur; yoksa/bozuksa boş dizin yayınlanır (çökme yok).
pub fn veri_oku(kok: &Path) -> Veri {
    match std::fs::read_to_string(kok.join("web/veri.json")) {
        Ok(metin) => Veri::json(&metin),
        Err(_) => Veri::default(),
    }
}

fn yanit_yaz(akis: &mut TcpStream, yanit: &Yanit) -> std::io::Result<()> {
    let sebep = match yanit.kod {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "OK",
    };
    let mut basliklar = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        yanit.kod,
        sebep,
        yanit.tur,
        yanit.govde.len()
    );
    for (ad, deger) in &yanit.ek_basliklar {
        basliklar.push_str(&format!("{ad}: {deger}\r\n"));
    }
    basliklar.push_str("\r\n");
    akis.write_all(basliklar.as_bytes())?;
    akis.write_all(&yanit.govde)
}

fn isle(mut akis: TcpStream, kok: &Path) -> std::io::Result<()> {
    let _ = akis.set_read_timeout(Some(Duration::from_millis(1500)));
    let _ = akis.set_write_timeout(Some(Duration::from_millis(5000)));
    let mut tampon = [0u8; 4096];
    let okunan = akis.read(&mut tampon)?;
    if okunan == 0 {
        return Ok(());
    }
    let metin = String::from_utf8_lossy(&tampon[..okunan]).to_string();
    let mut parcalar = metin.lines().next().unwrap_or_default().split_whitespace();
    let yontem = parcalar.next().unwrap_or("GET").to_uppercase();
    let yol = parcalar.next().unwrap_or("/");
    if yontem != "GET" && yontem != "HEAD" {
        return yanit_yaz(&mut akis, &Yanit::hata(405, "yalnız GET/HEAD"));
    }
    yanit_yaz(&mut akis, &yonlendir(kok, yol))
}

/// Sunucuyu bloğun sonuna kadar çalıştırır.
pub fn calistir(kok: PathBuf, port: u16) -> std::io::Result<()> {
    // canonical/OG adresleri gerçek porta göre üretilsin (varsayılan 8090
    // dışında bir port seçildiyse yanlış URL yazılmasın).
    if std::env::var("NEXUS_SITE_ADRES").is_err() {
        std::env::set_var("NEXUS_SITE_ADRES", format!("http://127.0.0.1:{port}"));
    }
    let dinleyici = TcpListener::bind(("127.0.0.1", port))?;
    println!(
        "[SEO] sunucu hazır: http://127.0.0.1:{port} · kök: {} · veri: web/veri.json",
        kok.display()
    );
    for akis in dinleyici.incoming().flatten() {
        let kok = kok.clone();
        std::thread::spawn(move || {
            let _ = isle(akis, &kok);
        });
    }
    Ok(())
}

fn main() {
    let mut argumanlar = std::env::args().skip(1);
    let port = argumanlar
        .next()
        .and_then(|deger| deger.parse::<u16>().ok())
        .unwrap_or(PORT);
    let kok = argumanlar
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if let Err(hata) = calistir(kok, port) {
        eprintln!("[!] sunucu başlatılamadı: {hata}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek() -> Site {
        Site {
            slug: "manisa-kebap-salonu".to_string(),
            ad: "Manisa Kebap Salonu".to_string(),
            sektor: "Restoran".to_string(),
            sektor_slug: "restoran".to_string(),
            il: "Manisa".to_string(),
            ilce: "Şehzadeler".to_string(),
            telefon: "0555 111 22 33".to_string(),
            adres: "Cumhuriyet Mah. 12. Sokak No:3".to_string(),
            aciklama: String::new(),
            baslik_oncelik: String::new(),
            sema: String::new(),
            medya: vec![
                Medya {
                    yol: "media/stock/restoran/ai_1.jpg".to_string(),
                    tur: "foto".to_string(),
                    kapak: String::new(),
                },
                Medya {
                    yol: "media/stock/restoran/hero.mp4".to_string(),
                    tur: "video".to_string(),
                    kapak: "media/stock/restoran/poster.jpg".to_string(),
                },
            ],
        }
    }

    #[test]
    fn sayfa_seo_etiketlerini_tasir() {
        let site = ornek();
        let html = sayfa(&site, "https://ornek.com");
        let baslik = baslik(&site);
        assert!(
            baslik.chars().count() <= 60,
            "başlık 60 karakteri aşıyor: {baslik}"
        );
        assert!(html.contains(&format!("<title>{baslik}</title>")));
        assert!(
            html.contains("<meta name=\"description\" content=\"Restoran · Manisa Kebap Salonu")
        );
        assert!(html
            .contains("<link rel=\"canonical\" href=\"https://ornek.com/s/manisa-kebap-salonu\">"));
        assert!(html.contains("property=\"og:title\""));
        assert!(html.contains("property=\"og:description\""));
        assert!(html.contains(
            "property=\"og:image\" content=\"https://ornek.com/media/stock/restoran/poster.jpg\""
        ));
        assert!(html.contains(
            "property=\"og:video\" content=\"https://ornek.com/media/stock/restoran/hero.mp4\""
        ));
        assert!(html.contains("<meta name=\"twitter:card\" content=\"summary_large_image\">"));
        assert!(html.contains("loading=\"lazy\""));
        assert!(html.contains("autoplay muted loop playsinline"));
        assert!(html.contains("application/ld+json"));
    }

    #[test]
    fn jsonld_sektore_gore_tip_secer() {
        let site = ornek();
        let html = sayfa(&site, "https://ornek.com");
        let bas = html.find("application/ld+json\">").unwrap() + "application/ld+json\">".len();
        let son = html[bas..].find("</script>").unwrap() + bas;
        let deger: serde_json::Value =
            serde_json::from_str(&html[bas..son]).expect("geçerli JSON-LD");
        assert_eq!(deger["@type"], "Restaurant");
        assert_eq!(deger["telephone"], "0555 111 22 33");
        assert_eq!(deger["address"]["addressCountry"], "TR");
        assert_eq!(sema_tipi("cekici"), "AutoRepair");
        assert_eq!(sema_tipi("bilinmeyen"), "LocalBusiness");
    }

    #[test]
    fn sitemap_ve_robots_tum_sayfalari_listeler() {
        let mut veri = Veri::default();
        veri.siteler.push(ornek());
        veri.siteler.push(Site {
            slug: "manisa-cekici".to_string(),
            ..ornek()
        });
        let harita = sitemap(&veri, "https://ornek.com/");
        assert!(harita.contains("<loc>https://ornek.com/s/manisa-kebap-salonu</loc>"));
        assert!(harita.contains("<loc>https://ornek.com/s/manisa-cekici</loc>"));
        assert!(harita.starts_with("<?xml"));
        let r = robots("https://ornek.com/");
        assert!(r.contains("Sitemap: https://ornek.com/sitemap.xml"));
    }

    #[test]
    fn medya_yolu_kacis_denemelerini_reddeder() {
        let kok = std::env::temp_dir().join(format!("nexus_seo_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&kok);
        std::fs::create_dir_all(kok.join("media/stock/restoran")).unwrap();
        std::fs::write(kok.join("media/stock/restoran/a.jpg"), [1u8; 4]).unwrap();
        std::fs::write(kok.join("gizli.txt"), [1u8; 4]).unwrap();

        assert!(medya_yolu(&kok, "/medya/restoran/a.jpg").is_some());
        assert!(medya_yolu(&kok, "/medya/../gizli.txt").is_none());
        assert!(medya_yolu(&kok, "/medya/restoran/../../gizli.txt").is_none());
        assert!(medya_yolu(&kok, "/medya/C:/Windows/win.ini").is_none());
        let _ = std::fs::remove_dir_all(&kok);
    }

    #[test]
    fn bozuk_veri_dosyasi_cokme_yerine_bos_dizin_verir() {
        assert!(Veri::json("{bozuk").siteler.is_empty());
        assert!(Veri::json("").siteler.is_empty());
        let veri = Veri::json(
            r#"{"surum":2,"guncelleme":"2026-11-06T23:00:00","siteler":[{"slug":"a","ad":"A","medya":[{"yol":"media/stock/x/1.jpg"}]}]}"#,
        );
        assert_eq!(veri.surum, 2);
        assert_eq!(veri.siteler.len(), 1);
        assert_eq!(veri.siteler[0].medya[0].tur, "foto");
    }

    #[test]
    fn html_kacisi_etiket_kacirmaz() {
        assert_eq!(kacis("<b>\"x\"&"), "&lt;b&gt;&quot;x&quot;&amp;");
        let mut site = ornek();
        site.ad = "<script>alert(1)</script>".to_string();
        let html = sayfa(&site, "https://ornek.com");
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn yonlendir_dizin_robots_sitemap_ve_404() {
        let kok = std::env::temp_dir().join(format!("nexus_seo_yol_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&kok);
        std::fs::create_dir_all(kok.join("web")).unwrap();
        std::fs::write(
            kok.join("web/veri.json"),
            r#"{"surum":1,"guncelleme":"2026-11-06T23:00:00","siteler":[{"slug":"a","ad":"A","sektor":"Restoran","sektor_slug":"restoran","il":"Manisa"}]}"#,
        )
        .unwrap();
        assert_eq!(yonlendir(&kok, "/").kod, 200);
        assert_eq!(yonlendir(&kok, "/robots.txt").kod, 200);
        assert_eq!(yonlendir(&kok, "/sitemap.xml").kod, 200);
        assert_eq!(yonlendir(&kok, "/s/a").kod, 200);
        assert_eq!(yonlendir(&kok, "/s/yok").kod, 404);
        assert_eq!(yonlendir(&kok, "/saglik").kod, 200);
        let _ = std::fs::remove_dir_all(&kok);
    }
}
