import io

p = "src/main.rs"
s = io.open(p, encoding="utf-8", newline="").read().replace("\r\n", "\n")

# 1) Sınıf/renk eşlemesi: Tailwind adı yerine gömülü CSS tonu.
eski = '''/// Puanın sözel karşılığı ve Tailwind renk adı.
fn karne_sinifi(puan: u8) -> (&'static str, &'static str) {
    match puan {
        80.. => ("İyi", "emerald"),
        60..=79 => ("Orta", "amber"),
        40..=59 => ("Zayıf", "orange"),
        _ => ("Kritik", "rose"),
    }
}'''
yeni = '''/// Puanın sözel karşılığı ve vurgu rengi (gömülü CSS değişkeni).
fn karne_sinifi(puan: u8) -> (&'static str, &'static str) {
    match puan {
        80.. => ("İyi", "#34d399"),
        60..=79 => ("Orta", "#fbbf24"),
        40..=59 => ("Zayıf", "#fb923c"),
        _ => ("Kritik", "#fb7185"),
    }
}

/// Karnenin gömülü stil sayfası. Harici CDN, script veya font isteği yok:
/// sayfa tek dosya olarak GitHub Pages / Netlify / Hostinger'a atılabilir.
const KARNE_CSS: &str = r#"
:root{color-scheme:dark;--zemin:#09090b;--kart:#18181b;--cizgi:#27272a;--metin:#f4f4f5;--soluk:#a1a1aa;--dim:#71717a;--amber:#fbbf24}
*{box-sizing:border-box}
html{-webkit-text-size-adjust:100%}
body{margin:0;background:radial-gradient(120% 90% at 50% -20%,rgba(251,191,36,.08),transparent 60%),var(--zemin);color:var(--metin);font-family:'Segoe UI',system-ui,-apple-system,sans-serif;line-height:1.55;-webkit-font-smoothing:antialiased}
a{color:inherit}
.kabuk{max-width:780px;margin:0 auto;padding:clamp(32px,6vw,56px) 20px}
.ust-etiket{margin:0;font-size:11px;font-weight:800;letter-spacing:.18em;text-transform:uppercase;color:var(--amber)}
h1{margin:10px 0 0;font-size:clamp(28px,5vw,40px);line-height:1.08;letter-spacing:-.02em}
.alt-bilgi{margin:6px 0 0;font-size:14px;color:var(--soluk)}
.kunye{margin:4px 0 0;font-size:12px;color:var(--dim)}
.kunye a{color:var(--soluk);text-decoration:none;border-bottom:1px solid var(--cizgi)}
.skor{display:grid;grid-template-columns:1fr;gap:14px;margin-top:30px}
.skor-kart{--ton:#fbbf24;border:1px solid color-mix(in srgb,var(--ton) 38%,transparent);background:color-mix(in srgb,var(--ton) 10%,transparent);border-radius:14px;padding:20px}
.skor-puan{margin:4px 0 0;font-size:52px;font-weight:900;line-height:1;color:var(--ton)}
.skor-alt{margin:0;font-size:12px;color:var(--soluk)}
.skor-sinif{margin:10px 0 0;font-size:14px;font-weight:800;color:var(--ton)}
.ozet-kart{border:1px solid var(--cizgi);background:rgba(24,24,27,.6);border-radius:14px;padding:20px}
.etiket{margin:0;font-size:11px;letter-spacing:.16em;text-transform:uppercase;color:var(--dim)}
.ozet{margin:8px 0 0;font-size:14px;color:#d4d4d8}
.ozet-sayi{margin:12px 0 0;font-size:12px;color:var(--dim)}
h2{margin:36px 0 0;font-size:18px;font-weight:800;letter-spacing:-.01em}
.liste{margin-top:12px;border:1px solid var(--cizgi);border-radius:14px;background:rgba(24,24,27,.45);overflow:hidden}
.satir{display:flex;align-items:flex-start;gap:12px;padding:16px}
.satir + .satir{border-top:1px solid var(--cizgi)}
.rozet{display:inline-flex;align-items:center;justify-content:center;flex:0 0 24px;width:24px;height:24px;margin-top:2px;border-radius:50%;border:1px solid;font-size:12px;font-weight:800}
.rozet.iyi{color:#6ee7b7;border-color:rgba(16,185,129,.45);background:rgba(16,185,129,.15)}
.rozet.orta{color:#fcd34d;border-color:rgba(245,158,11,.45);background:rgba(245,158,11,.15)}
.rozet.eksik{color:#fda4af;border-color:rgba(244,63,94,.45);background:rgba(244,63,94,.15)}
.satir-govde{min-width:0}
.satir-baslik{margin:0;font-size:14px;font-weight:700}
.durum{margin-left:4px;font-size:10px;font-weight:800;letter-spacing:.08em;text-transform:uppercase;color:var(--dim)}
.satir-deger{margin:2px 0 0;font-size:14px;color:var(--soluk);overflow-wrap:anywhere}
.adimlar{margin:12px 0 0;padding:0;list-style:none;display:grid;gap:10px;font-size:14px;color:#d4d4d8}
.adimlar li{display:flex;gap:12px;align-items:flex-start}
.adim-no{display:inline-flex;align-items:center;justify-content:center;flex:0 0 24px;width:24px;height:24px;border-radius:50%;background:#27272a;color:#fcd34d;font-size:12px;font-weight:800}
.demo-kart{margin-top:36px;border:1px solid rgba(245,158,11,.3);background:rgba(245,158,11,.06);border-radius:14px;padding:20px}
.demo-kart h2{margin:0;font-size:15px;color:#fcd34d}
.demo-kart p{margin:6px 0 0;font-size:13px;color:var(--soluk)}
.buton{display:inline-block;margin-top:14px;padding:10px 20px;border-radius:999px;background:var(--amber);color:#18181b;font-size:13px;font-weight:800;text-decoration:none}
.buton:hover{filter:brightness(1.06)}
.dipnot{margin-top:40px;padding-top:18px;border-top:1px solid var(--cizgi);font-size:12px;line-height:1.65;color:var(--dim)}
.dipnot p{margin:0}
.dipnot p + p{margin-top:8px}
@media(min-width:640px){.skor{grid-template-columns:1fr 2fr}.ozet-kart{grid-column:span 2}.kabuk{padding-left:28px;padding-right:28px}}
"#;'''
assert eski in s, "karne_sinifi"
s = s.replace(eski, yeni, 1)

# 2) Karne HTML: Tailwind sınıfları yerine gömülü CSS.
bas = s.index("fn karne_html(")
bit = s.index("\n/// Karneyi yazar.", bas)
fonksiyon = r'''fn karne_html(isletme: &BusinessLead, demo_dosyasi: Option<&str>) -> String {
    let (puan, satirlar) = karne_olc(isletme);
    let (sinif, ton) = karne_sinifi(puan);
    let ad = html_kacis(&isletme.name);
    let sektor = html_kacis(&isletme.category);
    let adres = html_kacis(&isletme.address);
    let kaynak = html_kacis(&isletme.kaynak);
    let tarih = bugun_anahtari();

    // Karne linki de WhatsApp'ta başlıklı ve açıklamalı görünsün.
    let paylasim = format!(
        concat!(
            "<meta name=\"description\" content=\"{aciklama}\">\n",
            "<meta property=\"og:type\" content=\"website\">\n",
            "<meta property=\"og:site_name\" content=\"{ad}\">\n",
            "<meta property=\"og:title\" content=\"{ad} — Dijital Görünürlük Karnesi ({sinif})\">\n",
            "<meta property=\"og:description\" content=\"{aciklama}\">\n",
            "<meta property=\"og:locale\" content=\"tr_TR\">\n",
            "<meta name=\"twitter:card\" content=\"summary\">\n",
            "<meta name=\"robots\" content=\"noindex\">"
        ),
        ad = ad,
        sinif = sinif,
        aciklama = format!(
            "{sektor} · Dijital görünürlük puanı {puan}/100. Eksikler ve atılacak adımlar tek sayfada."
        )
    );

    let eksikler: Vec<&KarneSatiri> = satirlar.iter().filter(|satir| satir.kayip > 0).collect();
    let eksik_sayisi = eksikler.len();
    let tamam_sayisi = satirlar.len() - eksik_sayisi;
    let ozet = if eksikler.is_empty() {
        "Bu karnedeki dört kontrolün hepsi geçildi; acil bir eksik görünmüyor.".to_string()
    } else {
        format!(
            "{} eksik tespit edildi: {}. Müşteri işletmenizi haritada ararken bu bilgileri göremediğinde doğrudan rakip işletmeye geçiyor.",
            eksikler.len(),
            eksikler
                .iter()
                .map(|satir| satir.baslik)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let satir_html: String = satirlar
        .iter()
        .map(|satir| {
            let (rozet, rozet_sinif) = match satir.durum {
                KarneDurumu::Iyi => ("✓", "iyi"),
                KarneDurumu::Orta => ("!", "orta"),
                KarneDurumu::Eksik => ("✕", "eksik"),
            };
            let etiket = match satir.durum {
                KarneDurumu::Iyi => "tamam",
                KarneDurumu::Orta => "kısmi",
                KarneDurumu::Eksik => "eksik",
            };
            format!(
                r##"<div class="satir">
<span class="rozet {rozet_sinif}">{rozet}</span>
<div class="satir-govde">
<p class="satir-baslik">{baslik} <span class="durum">{etiket}</span></p>
<p class="satir-deger">{deger}</p>
</div>
</div>
"##,
                baslik = html_kacis(satir.baslik),
                deger = html_kacis(&satir.deger),
            )
        })
        .collect();

    let adim_html: String = eksikler
        .iter()
        .enumerate()
        .map(|(sira, satir)| {
            format!(
                r##"<li><span class="adim-no">{}</span><span>{}</span></li>"##,
                sira + 1,
                html_kacis(satir.oneri)
            )
        })
        .collect();

    let adim_bolumu = if eksikler.is_empty() {
        String::new()
    } else {
        format!(
            r##"<h2>Önerilen adımlar</h2>
<ol class="adimlar">{adim_html}</ol>
"##
        )
    };

    let demo_blok = match demo_dosyasi {
        Some(dosya) => format!(
            r##"<section class="demo-kart">
<h2>Hazırlanan site taslağı</h2>
<p>Bu karnedeki eksikleri kapatan örnek sayfa sizin için hazırlandı.</p>
<a class="buton" href="{href}">Taslağı aç</a>
</section>
"##,
            href = html_kacis(dosya)
        ),
        None => String::new(),
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{ad} — Dijital Görünürlük Karnesi</title>
{paylasim}
<style>{css}</style>
</head>
<body>
<main class="kabuk">
<p class="ust-etiket">Dijital Görünürlük Karnesi</p>
<h1>{ad}</h1>
<p class="alt-bilgi">{sektor} · {adres}</p>
<p class="kunye">Rapor tarihi: {tarih} · Kaynak: <a href="{kaynak}" target="_blank" rel="noopener">işletme kaydı</a></p>

<section class="skor">
<div class="skor-kart" style="--ton:{ton}">
<p class="etiket">Dijital görünürlük puanı</p>
<p class="skor-puan">{puan}</p>
<p class="skor-alt">100 üzerinden</p>
<p class="skor-sinif">{sinif}</p>
</div>
<div class="ozet-kart">
<p class="etiket">Özet</p>
<p class="ozet">{ozet}</p>
<p class="ozet-sayi">{eksik_sayisi} eksik · {tamam_sayisi} tamam</p>
</div>
</section>

<h2>Tespitler</h2>
<div class="liste">
{satir_html}</div>
{adim_bolumu}
{demo_blok}

<footer class="dipnot">
<p>Bu karne, işletmenizin herkese açık kayıt bilgilerinden (Google Maps) otomatik olarak üretildi; hiçbir bilgi tahmin edilmedi. Eksik görünen bir bilgi varsa kaydınızı güncellemeniz karnedeki puanı da yükseltir.</p>
<p>Sektör: {sektor} · Kayıt: {kaynak}</p>
</footer>
</main>
</body>
</html>
"##,
        css = KARNE_CSS,
        paylasim = paylasim,
        ton = ton,
    )
}'''
s = s[:bas] + fonksiyon + s[bit:]

# 3) Regresyon testi: karne harici kaynak yüklemez.
eski = '''        assert!(html.contains("name=\\"robots\\" content=\\"noindex\\""));
    }
'''
yeni = '''        assert!(html.contains("name=\\"robots\\" content=\\"noindex\\""));
    }

    #[test]
    fn karne_harici_kaynak_yuklemez() {
        let html = karne_html(&ornek_isletme(), None);
        assert!(!html.contains("cdn.tailwindcss.com"), "CDN kalmamalı");
        assert!(!html.contains("<script src="), "harici script olmamalı");
        assert!(!html.contains("<link "), "harici stil olmamalı");
        assert!(html.contains("<style>"), "gömülü stil bulunmalı");
    }
'''
assert eski in s, "karne testi"
s = s.replace(eski, yeni, 1)

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("karne tek dosyaya cevrildi")
