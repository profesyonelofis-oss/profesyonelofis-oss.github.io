import io

# ---------------- main.rs ----------------
p = "src/main.rs"
s = io.open(p, encoding="utf-8", newline="").read().replace("\r\n", "\n")

s = s.replace("mod seo;\n", "mod seo;\nmod sektor_tasarim;\n", 1)

eski = '''    let sektor_vurgusu = hex_renk(sektor_rengi(&isletme.category));
    let vurgu = gecerli_renk(&ayar.demo_vurgu, &sektor_vurgusu);
    let zemin = gecerli_renk(&ayar.demo_zemin, DEMO_ZEMIN);
    let yazi = gecerli_renk(&ayar.demo_yazi, DEMO_YAZI);'''
yeni = '''    // Sektörel tasarım hafızası: palet, tipografi ve aciliyet seviyesi.
    let tasarim = sektor_tasarim::sec(&isletme.category);
    let sektor_vurgusu = if tasarim.vurgu.is_empty() {
        hex_renk(sektor_rengi(&isletme.category))
    } else {
        tasarim.vurgu.to_string()
    };
    let varsayilan_zemin = if tasarim.zemin.is_empty() {
        DEMO_ZEMIN
    } else {
        tasarim.zemin
    };
    let varsayilan_yazi = if tasarim.yazi.is_empty() {
        DEMO_YAZI
    } else {
        tasarim.yazi
    };
    let vurgu = gecerli_renk(&ayar.demo_vurgu, &sektor_vurgusu);
    let zemin = gecerli_renk(&ayar.demo_zemin, varsayilan_zemin);
    let yazi = gecerli_renk(&ayar.demo_yazi, varsayilan_yazi);'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''        tema: match ayar.demo_tema.as_str() {
            "gece" => "gece".to_string(),
            _ => String::new(),
        },
        ikincil: demo_metni(&ayar.demo_ikincil),'''
yeni = '''        tema: match ayar.demo_tema.as_str() {
            "gece" => "gece".to_string(),
            _ => String::new(),
        },
        tasarim: Some(demo_site::TasarimGirdi {
            baslik: tasarim.baslik.to_string(),
            govde: tasarim.govde.to_string(),
            aciliyet: tasarim.aciliyet,
            sinematik: tasarim.sinematik,
            rozetler: tasarim.rozetler.iter().map(|metin| (*metin).to_string()).collect(),
            cta: tasarim.cta.to_string(),
            vurgu2: tasarim.vurgu2.to_string(),
        }),
        ikincil: demo_metni(&ayar.demo_ikincil),'''
assert eski in s
s = s.replace(eski, yeni, 1)

io.open(p, "w", encoding="utf-8", newline="\n").write(s)

# ---------------- demo_site.rs ----------------
p = "src/demo_site.rs"
s = io.open(p, encoding="utf-8", newline="").read().replace("\r\n", "\n")

eski = '''pub struct Girdi {'''
yeni = '''/// Sektörel tasarım hafızasından gelen kimlik. Boş alan şablon varsayılanına
/// düşer; panelden yazılan metin her zaman önceliklidir.
#[derive(Clone, Debug, Default)]
pub struct TasarimGirdi {
    pub baslik: String,
    pub govde: String,
    pub aciliyet: u8,
    pub sinematik: bool,
    pub rozetler: Vec<String>,
    pub cta: String,
    pub vurgu2: String,
}

pub struct Girdi {'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''    pub font_css: &'static str,
}'''
yeni = '''    pub font_css: &'static str,
    /// Sektörün tasarım kimliği; `None` ise yalnız şablon varsayılanı.
    pub tasarim: Option<TasarimGirdi>,
}'''
assert eski in s
s = s.replace(eski, yeni, 1)

# tipografi
eski = '''    let baslik_yigin = if sahne {
        "'Cormorant Garamond','Instrument Serif',Georgia,'Times New Roman',serif"
    } else if serif {
        "'Instrument Serif','Space Grotesk',Georgia,'Times New Roman',serif"
    } else {
        "'Space Grotesk','Geist',system-ui,-apple-system,'Segoe UI',sans-serif"
    };'''
yeni = '''    let tasarim = g.tasarim.as_ref();
    let tasarim_baslik = tasarim
        .map(|t| t.baslik.trim().to_string())
        .filter(|metin| !metin.is_empty());
    let govde_yigin = tasarim
        .map(|t| t.govde.trim().to_string())
        .filter(|metin| !metin.is_empty())
        .unwrap_or_else(|| "'Geist',system-ui,-apple-system,'Segoe UI',sans-serif".to_string());
    let baslik_yigin = if sahne {
        "'Cormorant Garamond','Instrument Serif',Georgia,'Times New Roman',serif"
    } else if let Some(yigin) = tasarim_baslik.as_deref() {
        yigin
    } else if serif {
        "'Instrument Serif','Space Grotesk',Georgia,'Times New Roman',serif"
    } else {
        "'Space Grotesk','Geist',system-ui,-apple-system,'Segoe UI',sans-serif"
    };'''
assert eski in s
s = s.replace(eski, yeni, 1)

# cta
eski = '''    let cta = match g.cta.trim() {
        "" => ic.cta.to_string(),
        metin => metin.to_string(),
    };'''
yeni = '''    let cta = match g.cta.trim() {
        "" => tasarim
            .map(|t| t.cta.trim())
            .filter(|metin| !metin.is_empty())
            .unwrap_or(ic.cta)
            .to_string(),
        metin => metin.to_string(),
    };'''
assert eski in s
s = s.replace(eski, yeni, 1)

# acil + rozetler
eski = '''    let acil = acil_sektor(&g.sektor);'''
yeni = '''    let acil = acil_sektor(&g.sektor) || tasarim.is_some_and(|t| t.aciliyet >= 2);'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''    let hero_rozetler = if acil {
        format!(
            r##"<div class="hero-rozetler"><span class="hcip canli"><b></b>7/24 Aktif {sektor} Ekibi</span><span class="hcip">Ortalama Varış: 15–20 Dk</span><span class="hcip">Konum · tek dokunuş</span></div>"##
        )
    } else {
        String::new()
    };'''
yeni = '''    let hero_rozetler = if acil {
        let metinler: Vec<String> = tasarim
            .map(|t| t.rozetler.clone())
            .filter(|liste| !liste.is_empty())
            .unwrap_or_else(|| {
                vec![
                    format!("7/24 Aktif {sektor} Ekibi"),
                    "Ortalama Varış: 15–20 Dk".to_string(),
                    "Konum · tek dokunuş".to_string(),
                ]
            });
        let mut cips = String::new();
        for (sira, metin) in metinler.iter().take(3).enumerate() {
            if sira == 0 {
                cips.push_str(&format!(
                    r##"<span class="hcip canli"><b></b>{}</span>"##,
                    kacis(metin)
                ));
            } else {
                cips.push_str(&format!(
                    r##"<span class="hcip">{}</span>"##,
                    kacis(metin)
                ));
            }
        }
        format!(r##"<div class="hero-rozetler">{cips}</div>"##)
    } else {
        String::new()
    };'''
assert eski in s
s = s.replace(eski, yeni, 1)

# css degiskenleri
eski = '''        ":root{{--accent:{vurgu};--accent-karsi:{karsi};--bg:{zemin};--ink:{yazi};--r-1:8px;--r-2:14px;--r-3:18px;--r-4:26px;--f-baslik:{baslik};--f-govde:'Geist',system-ui,-apple-system,'Segoe UI',sans-serif}}\\n",'''
yeni = '''        ":root{{--accent:{vurgu};--accent-2:{vurgu2};--accent-karsi:{karsi};--bg:{zemin};--ink:{yazi};--r-1:8px;--r-2:14px;--r-3:18px;--r-4:26px;--f-baslik:{baslik};--f-govde:{govde}}}\\n",'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''        baslik = baslik_yigin,
        karsi = if parlak_mi(&g.vurgu) { "#12100e" } else { "#f8f5f0" }'''
yeni = '''        baslik = baslik_yigin,
        govde = govde_yigin,
        vurgu2 = tasarim
            .map(|t| t.vurgu2.trim())
            .filter(|metin| !metin.is_empty())
            .unwrap_or(g.vurgu.as_str()),
        karsi = if parlak_mi(&g.vurgu) { "#12100e" } else { "#f8f5f0" }'''
assert eski in s
s = s.replace(eski, yeni, 1)

# sinematik bolum
eski = '''    // Hakkında: fotoğraf varsa iki sütunlu, görselli düzen; yoksa başlık ve
    // metin ikiye ayrılır. Aynı fotoğraf galeride tekrar etmesin diye kare
    // galeriden ayrılıp buraya verilir (bkz. `demo_html`).'''
yeni = '''    // Sinematik kaydırma: iki tam genişlik kare, yavaş zoom + parallax ve
    // sırayla süzülen başlık. Fotoğraf yoksa gradyan "doku" karesi çizilir.
    let sinema = if !sahne && tasarim.is_some_and(|t| t.sinematik) {
        let kareler: Vec<&FotoVerisi> = g.foto.iter().chain(g.galeri.iter()).take(2).collect();
        let mut govde = String::new();
        let sinema_baslik = ozel(&g.metin.hakkinda_baslik, ic.hikaye_baslik);
        let sinema_metin = ozel(&g.metin.hakkinda1, ic.hikaye[0]);
        let bolumler = [(&sinema_baslik, &sinema_metin), (&bolum_metni, &ic.bolum_alt)];
        for (sira, (baslik, metin)) in bolumler.iter().enumerate() {
            let gorsel = match kareler.get(sira) {
                Some((mime, veri)) => format!(
                    r##"<div class="sinema-ic"><img src="data:{mime};base64,{veri}" alt="{ad} — {sektor}" loading="lazy" decoding="async"><div class="sinema-perde"></div></div>"##
                ),
                None => String::from(r##"<div class="sinema-ic doku"></div>"##),
            };
            govde.push_str(&format!(
                r##"<div class="sinema-kare" data-sinema style="--g:{gecikme}">
{gorsel}
<div class="sinema-yazi" data-rise style="--g:{gecikme}">
<span class="ince">{no} — {sektor} · {bolge}</span>
<h2>{baslik}</h2>
<p>{metin}</p>
</div>
</div>"##,
                gecikme = sira * 120,
                no = sira + 1
            ));
        }
        format!(r##"<section class="sinema" aria-label="Sinematik akış"><div class="kabuk sinema-kabuk">{govde}</div></section>"##)
    } else {
        String::new()
    };

    // Hakkında: fotoğraf varsa iki sütunlu, görselli düzen; yoksa başlık ve
    // metin ikiye ayrılır. Aynı fotoğraf galeride tekrar etmesin diye kare
    // galeriden ayrılıp buraya verilir (bkz. `demo_html`).'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''    if !sahne {
        sayfa.push_str(&format!(
        r##"<section class="bolum" id="hakkimizda">'''
yeni = '''    if !sahne {
        sayfa.push_str(&sinema);
        sayfa.push_str(&format!(
        r##"<section class="bolum" id="hakkimizda">'''
assert eski in s
s = s.replace(eski, yeni, 1)

# CSS: sinema stilleri
eski = '''@media (max-width:1000px){.asistan{right:12px;bottom:96px}.asistan-dugme{padding:11px 14px;font-size:12px}}
"#;'''
yeni = '''@media (max-width:1000px){.asistan{right:12px;bottom:96px}.asistan-dugme{padding:11px 14px;font-size:12px}}
/* --- sinematik kaydırma --- */
.sinema{padding:clamp(18px,3vw,34px) 0 clamp(6px,1.6vw,18px)}
.sinema-kabuk{display:grid;gap:clamp(14px,2.2vw,26px)}
.sinema-kare{position:relative;overflow:hidden;height:clamp(340px,72svh,620px);border-radius:var(--r-4);border:1px solid color-mix(in srgb,var(--accent) 22%,var(--cizgi));background:radial-gradient(120% 90% at 74% 12%,color-mix(in srgb,var(--accent) 30%,var(--bg)),var(--bg) 62%);isolation:isolate}
.sinema-ic{position:absolute;inset:-8% 0;transform:translateY(calc(var(--kaydir,0px) * .12));will-change:transform}
.sinema-ic img{width:100%;height:100%;object-fit:cover;object-position:50% 45%;animation:kenburns 32s ease-in-out infinite alternate;filter:saturate(1.02) contrast(1.04)}
.sinema-ic.doku{background:radial-gradient(80% 70% at 68% 26%,color-mix(in srgb,var(--accent) 44%,transparent),transparent 66%),repeating-linear-gradient(90deg,color-mix(in srgb,var(--ink) 6%,transparent) 0 1px,transparent 1px 92px)}
.sinema-perde{position:absolute;inset:0;background:linear-gradient(180deg,color-mix(in srgb,var(--bg) 12%,transparent),color-mix(in srgb,var(--bg) 78%,transparent) 94%);pointer-events:none}
.sinema-yazi{position:absolute;left:clamp(18px,4.4vw,64px);right:clamp(18px,4.4vw,64px);bottom:clamp(20px,4.6vw,58px);z-index:2;max-width:min(620px,90%)}
.sinema-yazi .ince{color:color-mix(in srgb,var(--accent-2) 82%,var(--ink));letter-spacing:.24em}
.sinema-yazi h2{font-family:var(--f-baslik);font-weight:600;font-size:clamp(26px,3.6vw,46px);line-height:1.06;letter-spacing:-.02em;margin:10px 0 0;color:var(--ink)}
.sinema-yazi p{margin:12px 0 0;font-size:clamp(13px,1.15vw,15.5px);line-height:1.7;color:color-mix(in srgb,var(--ink) 70%,transparent);max-width:52ch}
.sinema-kare::after{content:"";position:absolute;left:0;right:0;bottom:0;height:1px;background:linear-gradient(90deg,transparent,var(--accent-2),transparent);opacity:.55;z-index:3}
@media (max-width:1000px){.sinema-kare{height:clamp(300px,62svh,460px);border-radius:var(--r-3)}.sinema-yazi{left:18px;right:18px;bottom:18px}}
@media (prefers-reduced-motion:reduce){.sinema-ic{transform:none}.sinema-ic img{animation:none}}
"#;'''
assert eski in s
s = s.replace(eski, yeni, 1)

# JS: sinema kareleri de parallax listesine
s = s.replace(
    'document.querySelectorAll(".sahne.foto, .galeri-kare")',
    'document.querySelectorAll(".sahne.foto, .galeri-kare, .sinema-kare")',
)

# test girdisi
eski = '''            satici_tel: None,
            font_css: "",
        }'''
yeni = '''            satici_tel: None,
            font_css: "",
            tasarim: None,
        }'''
assert eski in s
s = s.replace(eski, yeni, 1)

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("tasarim yamalandi")
