from pathlib import Path

kok = Path('.')
yapilan = []

def yaz(yol, eski, yeni):
    p = kok / yol
    s = p.read_text(encoding='utf-8')
    if eski not in s:
        raise SystemExit(f'BULUNAMADI ({yol}): ' + eski[:100])
    s = s.replace(eski, yeni, 1)
    p.write_text(s, encoding='utf-8')
    yapilan.append(f'{yol}: {eski[:45]}')

# ======================= demo_site.rs =====================================
# 1) Tohumdan türeyen görsel dünya tipleri ve yardımcıları
YARDIMCI = r'''
/// İşletmeye özel sinematik arka plan dünyası: bütün alanlar işletme adı ve
/// adresinden türeyen tohumdan gelir. Aynı sektörde iki işletme asla aynı
/// akışı, aynı hızı ve aynı partikül yoğunluğunu almaz.
#[derive(Clone, Debug)]
pub struct Dunya {
    pub tohum: u64,
    pub aci: u32,
    pub aci2: u32,
    /// Akışkan dalganın hız çarpanı (0,55–1,45).
    pub hiz: f64,
    /// Dalga frekansı çarpanı (0,72–1,58).
    pub dalga: f64,
    /// Partikül yoğunluğu (0,16–0,78).
    pub parca: f64,
    /// Vinyet koyuluğu (0,34–0,66).
    pub vinyet: f64,
    /// Birinci aurora rengi (vurgudan türetilir).
    pub akis1: String,
    /// İkinci aurora rengi (zemine doğru karıştırılır).
    pub akis2: String,
}

/// Tohumdan deterministik dünya üretir; aynı işletme her üretimde aynı
/// görsel kimliği, komşu işletme farklı kimliği alır.
pub fn dunya_uret(tohum: &str, vurgu: &str, zemin: &str) -> Dunya {
    let sayi = u64::from_str_radix(tohum.trim(), 16).unwrap_or(0x5eed_1234);
    let aci = (sayi % 360) as u32;
    let aci2 = ((sayi >> 7) % 360) as u32;
    let hiz = 0.55 + ((sayi >> 13) % 100) as f64 / 100.0 * 0.9;
    let dalga = 0.72 + ((sayi >> 21) % 100) as f64 / 100.0 * 0.86;
    let parca = 0.16 + ((sayi >> 29) % 100) as f64 / 100.0 * 0.62;
    let vinyet = 0.34 + ((sayi >> 37) % 100) as f64 / 100.0 * 0.32;
    let akis1 = renk_karistir(vurgu, &renk_dondur(vurgu, f64::from(aci) * 0.6), 0.55);
    let akis2 = renk_karistir(
        zemin,
        &renk_dondur(vurgu, 180.0 + f64::from(aci2) * 0.5),
        0.62,
    );
    Dunya {
        tohum: sayi,
        aci,
        aci2,
        hiz,
        dalga,
        parca,
        vinyet,
        akis1,
        akis2,
    }
}

/// `#rrggbb` (veya `#rgb`) rengi 0–1 aralığına çevirir.
fn renk_bilesenleri(hex: &str) -> Option<[f64; 3]> {
    let temiz = hex.trim().trim_start_matches('#');
    let genis = if temiz.len() == 3 {
        temiz
            .chars()
            .flat_map(|k| [k, k])
            .collect::<String>()
    } else if temiz.len() == 6 {
        temiz.to_string()
    } else {
        return None;
    };
    let parca = |bas: usize| u8::from_str_radix(&genis[bas..bas + 2], 16).ok();
    Some([
        f64::from(parca(0)?) / 255.0,
        f64::from(parca(2)?) / 255.0,
        f64::from(parca(4)?) / 255.0,
    ])
}

fn renk_yaz(rgb: [f64; 3]) -> String {
    let kanal = |deger: f64| (deger.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", kanal(rgb[0]), kanal(rgb[1]), kanal(rgb[2]))
}

/// İki rengi orana göre karıştırır (`oran` ikinci rengin payı).
fn renk_karistir(a: &str, b: &str, oran: f64) -> String {
    let (Some(a), Some(b)) = (renk_bilesenleri(a), renk_bilesenleri(b)) else {
        return a.to_string();
    };
    let oran = oran.clamp(0.0, 1.0);
    renk_yaz([
        a[0] * (1.0 - oran) + b[0] * oran,
        a[1] * (1.0 - oran) + b[1] * oran,
        a[2] * (1.0 - oran) + b[2] * oran,
    ])
}

/// Rengin tonunu derece kadar döndürür (HSL üzerinden, doygunluk korunur).
fn renk_dondur(hex: &str, derece: f64) -> String {
    let Some([r, g, b]) = renk_bilesenleri(hex) else {
        return hex.to_string();
    };
    let mak = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (mak + min) / 2.0;
    let d = mak - min;
    if d < f64::EPSILON {
        return hex.to_string();
    }
    let s = if l > 0.5 { d / (2.0 - mak - min) } else { d / (mak + min) };
    let mut h = if (mak - r).abs() < f64::EPSILON {
        ((g - b) / d) % 6.0
    } else if (mak - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h = (h * 60.0 + derece).rem_euclid(360.0) / 360.0;
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let kanal = |t: f64| {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    renk_yaz([kanal(h + 1.0 / 3.0), kanal(h), kanal(h - 1.0 / 3.0)])
}
'''
yaz(
    'src/demo_site.rs',
    '/// Sahnede duran 3D nesne. Hepsi saf CSS/SVG/Canvas; görsel dosyası yok.',
    YARDIMCI + '\n/// Sahnede duran 3D nesne. Hepsi saf CSS/SVG/Canvas; görsel dosyası yok.',
)

# 2) Dünya hesabı + main etiketine değişkenler
yaz(
    'src/demo_site.rs',
    '    let mut sayfa = String::with_capacity(64 * 1024);',
    '    // Prosedürel görsel dünya: işletme tohumundan türeyen renk/hız/partikül.\n'
    '    let dunya = dunya_uret(&tohum, &vurgu, &zemin);\n'
    '    let mut sayfa = String::with_capacity(64 * 1024);',
)
yaz(
    'src/demo_site.rs',
    '<main id="sn-icerik" class="sn-akis" data-stil="{stil}" data-nesne="{nesne}" data-hareket="{hareket}" data-tohum="{tohum}">{sahneler}</main>',
    '<main id="sn-icerik" class="sn-akis" data-stil="{stil}" data-nesne="{nesne}" data-hareket="{hareket}" data-tohum="{tohum}" data-dunya="{dunya_tohum}" style="--sn-akis1:{akis1};--sn-akis2:{akis2};--sn-hiz:{hiz};--sn-dalga:{dalga};--sn-parca:{parca};--sn-vinyet:{vinyet};--sn-aci:{aci}">{sahneler}</main>',
)
yaz(
    'src/demo_site.rs',
    '        js = SINEMATIK_JS.replace("__PROGRAM__", program_js),\n    ));',
    '        js = SINEMATIK_JS.replace("__PROGRAM__", program_js),\n'
    '        dunya_tohum = dunya.tohum,\n'
    '        akis1 = dunya.akis1,\n'
    '        akis2 = dunya.akis2,\n'
    '        hiz = format!("{:.2}", dunya.hiz),\n'
    '        dalga = format!("{:.2}", dunya.dalga),\n'
    '        parca = format!("{:.2}", dunya.parca),\n'
    '        vinyet = format!("{:.2}", dunya.vinyet),\n'
    '        aci = dunya.aci,\n'
    '    ));',
)

# 3) CSS: aurora katmanı, tohumlu vinyet, partikül dokusu
yaz(
    'src/demo_site.rs',
    '.sn-akis{position:relative;z-index:2}\n',
    '.sn-akis{position:relative;z-index:2}\n'
    '/* Aurora: işletme tohumundan gelen iki renk, hız ve yoğunluk. */\n'
    '.sn-akis::before{content:"";position:fixed;inset:-18%;z-index:-1;pointer-events:none;'
    'background:radial-gradient(42% 42% at 24% 28%,color-mix(in srgb,var(--sn-akis1,var(--sn-vurgu)) 62%,transparent),transparent 70%),'
    'radial-gradient(48% 48% at 76% 66%,color-mix(in srgb,var(--sn-akis2,var(--sn-vurgu)) 52%,transparent),transparent 72%),'
    'radial-gradient(38% 38% at 52% 88%,color-mix(in srgb,var(--sn-akis1,var(--sn-vurgu)) 30%,transparent),transparent 74%);'
    'filter:blur(46px);opacity:calc(.28 + var(--sn-parca,.35)*.4);'
    'animation:snAkis calc(26s / var(--sn-hiz,1)) ease-in-out infinite alternate;will-change:transform}\n'
    'html[data-hareket="statik"] .sn-akis::before{animation:none}\n'
    '.sn-sahne::before{background-image:radial-gradient(35% 40% at calc(10% + var(--sn-aci,0) * .2%) 8%,color-mix(in srgb,var(--sn-akis1,var(--sn-vurgu)) 16%,transparent),transparent 62%)}\n',
)
yaz(
    'src/demo_site.rs',
    '.sn-vinyet{position:fixed;inset:0;z-index:1;pointer-events:none;background:radial-gradient(120% 100% at 50% 38%,transparent 44%,rgba(0,0,0,.5) 100%)}',
    '.sn-vinyet{position:fixed;inset:0;z-index:1;pointer-events:none;background:radial-gradient(calc(88% + var(--sn-vinyet,.5) * 44%) 100% at 50% 38%,transparent calc(30% + var(--sn-vinyet,.5) * 22%),rgba(0,0,0,var(--sn-vinyet,.5)) 100%)}',
)
yaz(
    'src/demo_site.rs',
    '@keyframes snHarf{to{transform:translateY(0) rotate(0);opacity:1}}',
    '@keyframes snAkis{0%{transform:translate3d(-3%,-2%,0) scale(1.02) rotate(-2deg)}50%{transform:translate3d(2%,1%,0) scale(1.07) rotate(1deg)}100%{transform:translate3d(4%,-3%,0) scale(1.03) rotate(2deg)}}\n@keyframes snHarf{to{transform:translateY(0) rotate(0);opacity:1}}',
)

# 4) Shader: tohumlu uniform'lar
yaz(
    'src/demo_site.rs',
    '      var fs="precision mediump float;uniform vec2 u_res;uniform vec2 u_m;uniform float u_t;uniform vec3 u_r;void main(){vec2 q=(gl_FragCoord.xy-.5*u_res)/u_res.y;float d=length(q-vec2(u_m.x-.5,u_m.y-.5));float h=sin(q.x*2.1+u_t*.22)+sin(q.y*2.7-u_t*.17)+sin((q.x+q.y)*1.3+u_t*.11);vec3 c=mix(vec3(.02,.024,.034),vec3(.06,.07,.095),.5+.26*h);c+=exp(-d*2.2)*.9*u_r;float g=fract(sin(dot(gl_FragCoord.xy,vec2(12.9898,78.233))+u_t*.6)*43758.5453);c+=(g-.5)*.05;gl_FragColor=vec4(c,1.);}";',
    '      var fs="precision mediump float;uniform vec2 u_res;uniform vec2 u_m;uniform float u_t;uniform vec3 u_r;uniform vec3 u_r2;uniform float u_hiz;uniform float u_dalga;uniform float u_parca;void main(){vec2 q=(gl_FragCoord.xy-.5*u_res)/u_res.y;float tt=u_t*u_hiz;float d=length(q-vec2(u_m.x-.5,u_m.y-.5));float h=sin(q.x*2.1*u_dalga+tt*.22)+sin(q.y*2.7*u_dalga-tt*.17)+sin((q.x+q.y)*1.3*u_dalga+tt*.11);vec3 c=mix(vec3(.02,.024,.034),vec3(.06,.07,.095),.5+.26*h);c+=exp(-d*2.2)*.9*u_r;c+=exp(-length(q-vec2(.42,.32))*1.7)*.55*u_r2;float p=fract(sin(dot(floor(gl_FragCoord.xy/3.),vec2(12.9898,78.233))+tt*.4)*43758.5453);c+=step(1.-u_parca*.2,p)*u_r*.08;float g=fract(sin(dot(gl_FragCoord.xy,vec2(12.9898,78.233))+tt*.6)*43758.5453);c+=(g-.5)*.05;gl_FragColor=vec4(c,1.);}";',
)
yaz(
    'src/demo_site.rs',
    '        var uRes=gl.getUniformLocation(p,"u_res"),uT=gl.getUniformLocation(p,"u_t"),uM=gl.getUniformLocation(p,"u_m"),uR=gl.getUniformLocation(p,"u_r");',
    '        var uRes=gl.getUniformLocation(p,"u_res"),uT=gl.getUniformLocation(p,"u_t"),uM=gl.getUniformLocation(p,"u_m"),uR=gl.getUniformLocation(p,"u_r"),uR2=gl.getUniformLocation(p,"u_r2"),uH=gl.getUniformLocation(p,"u_hiz"),uD=gl.getUniformLocation(p,"u_dalga"),uP=gl.getUniformLocation(p,"u_parca");',
)
yaz(
    'src/demo_site.rs',
    '        gl.uniform3f(uR,cr*.75,cg*.55,cb*.35);',
    '        gl.uniform3f(uR,cr*.75,cg*.55,cb*.35);\n'
    '        /* Tohumlu arka plan: ikinci renk, hız, dalga ve partikül yoğunluğu\n'
    '           işletmenin CSS değişkenlerinden okunur. */\n'
    '        var stil=getComputedStyle(K);\n'
    '        var oku=function(ad,v){var n=parseFloat(stil.getPropertyValue(ad));return isNaN(n)?v:n;};\n'
    '        var renk2=stil.getPropertyValue("--sn-akis2").trim()||renk;\n'
    '        var m2=renk2.replace("#","");\n'
    '        if(m2.length===3)m2=m2[0]+m2[0]+m2[1]+m2[1]+m2[2]+m2[2];\n'
    '        var r2r=parseInt(m2.substr(0,2),16)/255||.28,r2g=parseInt(m2.substr(2,2),16)/255||.36,r2b=parseInt(m2.substr(4,2),16)/255||.5;\n'
    '        gl.uniform3f(uR2,r2r,r2g,r2b);\n'
    '        gl.uniform1f(uH,oku("--sn-hiz",1));\n'
    '        gl.uniform1f(uD,oku("--sn-dalga",1));\n'
    '        gl.uniform1f(uP,oku("--sn-parca",.35));',
)

# 5) Testler
yaz(
    'src/demo_site.rs',
    '    #[test]\n    fn sinematik_premium_katman_taslari_basilir() {',
    '    #[test]\n'
    '    fn dunya_tohumdan_tutarli_ve_benzersiz() {\n'
    '        let a = dunya_uret("a1b2c3", "#c9a227", "#08080a");\n'
    '        let b = dunya_uret("a1b2c3", "#c9a227", "#08080a");\n'
    '        let c = dunya_uret("0f0f0f", "#c9a227", "#08080a");\n'
    '        assert_eq!(a.akis1, b.akis1, "aynı işletme aynı dünyayı almalı");\n'
    '        assert_eq!(a.hiz, b.hiz);\n'
    '        assert_ne!(a.akis1, c.akis1, "komşu işletme farklı renk almalı");\n'
    '        assert!(a.hiz >= 0.55 && a.hiz <= 1.45);\n'
    '        assert!(a.parca >= 0.16 && a.parca <= 0.78);\n'
    '        assert!(a.akis1.starts_with(\'#\') && a.akis1.len() == 7);\n'
    '    }\n\n'
    '    #[test]\n'
    '    fn sinematik_dunya_sayfaya_gomulur_ve_isletmeye_gore_degisir() {\n'
    '        let mut a = sinematik_girdi(6);\n'
    '        a.ad = "Narin Ocak".to_string();\n'
    '        a.adres = "Cihangir, Beyoğlu / İstanbul".to_string();\n'
    '        let mut b = a.clone();\n'
    '        b.ad = "Vita Ağız ve Diş Sağlığı".to_string();\n'
    '        b.adres = "Kızılay, Çankaya / Ankara".to_string();\n'
    '        let sa = uret(&a);\n'
    '        let sb = uret(&b);\n'
    '        let renk_al = |html: &str| {\n'
    '            let bas = html.find("--sn-akis1:").expect("aurora rengi") + 12;\n'
    '            html[bas..bas + 7].to_string()\n'
    '        };\n'
    '        assert_ne!(renk_al(&sa), renk_al(&sb), "iki işletme aynı aurorayı almamalı");\n'
    '        for parca in ["data-dunya=", "--sn-parca:", "--sn-hiz:", "u_parca", "snAkis"] {\n'
    '            assert!(sa.contains(parca), "eksik: {parca}");\n'
    '        }\n'
    '    }\n\n'
    '    #[test]\n'
    '    fn sinematik_premium_katman_taslari_basilir() {',
)

# ======================= sektor_medya.rs: dış kaynak kapalı ================
yaz(
    'src/sektor_medya.rs',
    '/// Onaylanan medyanın kök klasörü.\npub const KLASOR: &str = "media/stock";',
    '/// Dış medya kaynakları kapalı: arama, indirme ve stok kullanımı tamamen\n'
    '/// devre dışı. Görseller kod tabanlı (tohumlu shader/CSS) üretilir.\n'
    'pub const DIS_KAYNAK_ACIK: bool = false;\n'
    '/// Onaylanan medyanın kök klasörü.\npub const KLASOR: &str = "media/stock";',
)
yaz(
    'src/sektor_medya.rs',
    'pub fn tara(sektor: &SektorMedya) -> SektorMedya {\n    let mut sonuc = sektor.clone();',
    'pub fn tara(sektor: &SektorMedya) -> SektorMedya {\n'
    '    if !DIS_KAYNAK_ACIK {\n'
    '        let mut kapali = sektor.clone();\n'
    '        kapali.asama = Asama::Hata;\n'
    '        kapali.hata = "Dış medya kapalı: görseller kod tabanlı üretiliyor.".to_string();\n'
    '        return kapali;\n'
    '    }\n'
    '    let mut sonuc = sektor.clone();',
)
yaz(
    'src/sektor_medya.rs',
    'pub fn onayla(sm: &SektorMedya) -> Result<Vec<String>, String> {\n    let kok = Path::new(KLASOR).join(&sm.slug);',
    'pub fn onayla(sm: &SektorMedya) -> Result<Vec<String>, String> {\n'
    '    if !DIS_KAYNAK_ACIK {\n'
    '        return Err("Dış medya kapalı: indirme yapılmaz.".to_string());\n'
    '    }\n'
    '    let kok = Path::new(KLASOR).join(&sm.slug);',
)
yaz(
    'src/sektor_medya.rs',
    'pub fn video_yoldan(kok: &Path, slug: &str) -> Option<StokVideo> {\n    let deger = manifest_oku_yoldan(kok);',
    'pub fn video_yoldan(kok: &Path, slug: &str) -> Option<StokVideo> {\n'
    '    if !DIS_KAYNAK_ACIK {\n'
    '        return None;\n'
    '    }\n'
    '    let deger = manifest_oku_yoldan(kok);',
)
yaz(
    'src/sektor_medya.rs',
    'pub fn fotolar_yoldan(kok: &Path, slug: &str) -> Vec<(String, String)> {\n    let deger = manifest_oku_yoldan(kok);',
    'pub fn fotolar_yoldan(kok: &Path, slug: &str) -> Vec<(String, String)> {\n'
    '    if !DIS_KAYNAK_ACIK {\n'
    '        return Vec::new();\n'
    '    }\n'
    '    let deger = manifest_oku_yoldan(kok);',
)
yaz(
    'src/sektor_medya.rs',
    'pub fn onayli_medya_sayisi() -> usize {\n    let deger = manifest_oku_yoldan(Path::new("."));',
    'pub fn onayli_medya_sayisi() -> usize {\n'
    '    if !DIS_KAYNAK_ACIK {\n'
    '        return 0;\n'
    '    }\n'
    '    let deger = manifest_oku_yoldan(Path::new("."));',
)
yaz(
    'src/sektor_medya.rs',
    '    #[test]\n    fn api_varsayilan_olarak_kapali() {',
    '    #[test]\n'
    '    fn dis_medya_varsayilan_olarak_kapali() {\n'
    '        assert!(!DIS_KAYNAK_ACIK, "harici stok medya kapalı olmalı");\n'
    '        let mut sm = SektorMedya::yeni("Kuaför", "Güzellik & Bakım");\n'
    '        sm.asama = Asama::Bitti;\n'
    '        sm.foto = vec![Aday::default()];\n'
    '        sm.video = vec![Aday::default()];\n'
    '        assert!(fotolar_yoldan(&gecici("kapali"), "kuafor").is_empty());\n'
    '        assert!(video_yoldan(&gecici("kapali"), "kuafor").is_none());\n'
    '        assert!(onayla(&sm).is_err(), "indirme kapalı olmalı");\n'
    '        assert_eq!(onayli_medya_sayisi(), 0);\n'
    '    }\n\n'
    '    #[test]\n'
    '    fn api_varsayilan_olarak_kapali() {',
)

# ======================= stok_video.rs: Ken Burns =========================
yaz(
    'src/stok_video.rs',
    '/// `KAYNAK.json` dosyasını klasöre yazar.',
    '/// Müşteri fotoğrafını Ken Burns (zoom + pan) sinematik döngüye çevirir:\n'
    '/// 720p, sessiz, 8 sn, her kare anahtar kare; poster de üretilir.\n'
    '/// Dış kaynak kullanılmaz, yalnız yerel dosya ve ffmpeg gerekir.\n'
    'pub fn foto_ken_burns(grup_id: &str, kaynak_yol: &str) -> Result<Hazir, String> {\n'
    '    if !ffmpeg_var() {\n'
    '        return Err("ffmpeg bulunamadı: fotoğraf hareketlendirilemiyor.".to_string());\n'
    '    }\n'
    '    let kaynak = Path::new(kaynak_yol.trim());\n'
    '    if !kaynak.is_file() {\n'
    '        return Err("Fotoğraf bulunamadı: dosya yolunu kontrol edin.".to_string());\n'
    '    }\n'
    '    let uzanti = kaynak\n'
    '        .extension()\n'
    '        .and_then(|uzanti| uzanti.to_str())\n'
    '        .map(|uzanti| uzanti.to_ascii_lowercase())\n'
    '        .unwrap_or_default();\n'
    '    if !matches!(uzanti.as_str(), "jpg" | "jpeg" | "png" | "webp") {\n'
    '        return Err("Yalnız jpg, jpeg, png veya webp fotoğraf kullanılabilir.".to_string());\n'
    '    }\n'
    '    let klasor = Path::new(MEDYA_KOK).join(grup_id);\n'
    '    std::fs::create_dir_all(&klasor)\n'
    '        .map_err(|hata| format!("{} oluşturulamadı: {hata}", klasor.display()))?;\n'
    '    let video = klasor.join("hero.mp4");\n'
    '    let poster = klasor.join("poster.webp");\n'
    '    // Önce büyüt, sonra zoompan: 720p çıktıda titreme olmaz.\n'
    '    let filtre = "scale=2560:-2,zoompan=z=\'min(1.0+0.0011*on,1.28)\':x=\'iw/2-(iw/zoom)/2+sin(on/70)*18\':y=\'ih/2-(ih/zoom)/2-cos(on/90)*14\':d=200:s=1280x720:fps=25,format=yuv420p";\n'
    '    let durum = std::process::Command::new("ffmpeg")\n'
    '        .args(["-y", "-v", "error", "-loop", "1", "-i"])\n'
    '        .arg(kaynak)\n'
    '        .args([\n'
    '            "-t",\n'
    '            "8",\n'
    '            "-vf",\n'
    '            filtre,\n'
    '            "-an",\n'
    '            "-c:v",\n'
    '            "libx264",\n'
    '            "-preset",\n'
    '            "medium",\n'
    '            "-crf",\n'
    '            "23",\n'
    '            "-g",\n'
    '            "1",\n'
    '            "-keyint_min",\n'
    '            "1",\n'
    '            "-sc_threshold",\n'
    '            "0",\n'
    '            "-movflags",\n'
    '            "+faststart",\n'
    '        ])\n'
    '        .arg(&video)\n'
    '        .status()\n'
    '        .map_err(|hata| format!("ffmpeg çalıştırılamadı: {hata}"))?;\n'
    '    if !durum.success() {\n'
    '        return Err("Fotoğraf hareketli videoya çevrilemedi.".to_string());\n'
    '    }\n'
    '    let durum = std::process::Command::new("ffmpeg")\n'
    '        .args(["-y", "-v", "error", "-ss", "0.5", "-i"])\n'
    '        .arg(&video)\n'
    '        .args([\n'
    '            "-frames:v",\n'
    '            "1",\n'
    '            "-vf",\n'
    '            "scale=-2:720",\n'
    '            "-c:v",\n'
    '            "libwebp",\n'
    '            "-quality",\n'
    '            "82",\n'
    '        ])\n'
    '        .arg(&poster)\n'
    '        .status()\n'
    '        .map_err(|hata| format!("ffmpeg (poster) çalıştırılamadı: {hata}"))?;\n'
    '    if !durum.success() {\n'
    '        return Err("Poster görüntüsü çıkarılamadı.".to_string());\n'
    '    }\n'
    '    let video_bayt = std::fs::metadata(&video).map(|m| m.len()).unwrap_or(0);\n'
    '    let poster_bayt = std::fs::metadata(&poster).map(|m| m.len()).unwrap_or(0);\n'
    '    let bilgi = serde_json::json!({\n'
    '        "kaynak": "müşteri fotoğrafı",\n'
    '        "dosya": "hero.mp4",\n'
    '        "poster": "poster.webp",\n'
    '        "efekt": "ken-burns",\n'
    '        "sure_sn": VIDEO_SN,\n'
    '        "genislik": 1280,\n'
    '        "yukseklik": 720,\n'
    '        "video_bayt": video_bayt,\n'
    '        "poster_bayt": poster_bayt,\n'
    '        "tarih": tarih_metni(),\n'
    '    });\n'
    '    kaynak_yaz(&klasor, &bilgi)?;\n'
    '    Ok(Hazir {\n'
    '        grup: grup_id.to_string(),\n'
    '        dosya_adi: "hero.mp4".to_string(),\n'
    '        poster_adi: "poster.webp".to_string(),\n'
    '        video_bayt,\n'
    '        poster_bayt,\n'
    '        atif: "Müşteri fotoğrafı".to_string(),\n'
    '        sure: VIDEO_SN,\n'
    '    })\n'
    '}\n\n'
    '/// `KAYNAK.json` dosyasını klasöre yazar.',
)
yaz(
    'src/stok_video.rs',
    '    fn api_varsayilan_olarak_kapali() {',
    '    #[test]\n'
    '    #[ignore]\n'
    '    fn foto_ken_burns_uretir() {\n'
    '        let kaynak = "media/stock/kuafor/photos/1.jpg";\n'
    '        if !Path::new(kaynak).is_file() {\n'
    '            println!("örnek fotoğraf yok; test atlandı");\n'
    '            return;\n'
    '        }\n'
    '        let hazir = foto_ken_burns("deneme_kb", kaynak).expect("ken burns");\n'
    '        println!("video {} B · poster {} B", hazir.video_bayt, hazir.poster_bayt);\n'
    '        assert!(hazir.video_bayt > 40_000, "video üretilmedi");\n'
    '        assert!(hazir.poster_bayt > 2_000, "poster üretilmedi");\n'
    '        assert!(Path::new("media/sektor/deneme_kb/KAYNAK.json").is_file());\n'
    '        let _ = std::fs::remove_dir_all("media/sektor/deneme_kb");\n'
    '    }\n\n'
    '    fn api_varsayilan_olarak_kapali() {',
)

# ======================= main.rs: panel + alan ============================
yaz(
    'src/main.rs',
    '    /// Stok video bulucu (yerel içe aktarma): seçili grup, dosya yolu,',
    '    /// Müşteri fotoğrafından Ken Burns döngüsü için dosya yolu.\n'
    '    stok_foto_yol: String,\n'
    '    /// Stok video bulucu (yerel içe aktarma): seçili grup, dosya yolu,',
)
yaz(
    'src/main.rs',
    '            stok_grup: stok_video::GRUPLAR[0].id.to_string(),',
    '            stok_foto_yol: String::new(),\n'
    '            stok_grup: stok_video::GRUPLAR[0].id.to_string(),',
)
yaz(
    'src/main.rs',
    '        // Sürükle-bırak: pencereye bırakılan dosya yol alanına düşer.',
    '        // Fotoğraftan sinematik döngü: yerel dosya, ffmpeg, dış kaynak yok.\n'
    '        ui.add_space(6.0);\n'
    '        ui.label(\n'
    '            egui::RichText::new(\n'
    '                "Fotoğraftan sinematik döngü · Ken Burns zoom-pan · 720p · 8 sn · sessiz",\n'
    '            )\n'
    '            .size(9.8)\n'
    '            .color(DIM),\n'
    '        );\n'
    '        ui.horizontal(|ui| {\n'
    '            ui.add(\n'
    '                egui::TextEdit::singleline(&mut self.stok_foto_yol)\n'
    '                    .hint_text("C:\\\\fotoğraflar\\\\cephe.jpg")\n'
    '                    .desired_width(190.0),\n'
    '            );\n'
    '            if ui\n'
    '                .add_enabled(!self.stok_mesgul, egui::Button::new("Hareketlendir"))\n'
    '                .clicked()\n'
    '            {\n'
    '                let grup = self.stok_grup.clone();\n'
    '                let yol = self.stok_foto_yol.trim().to_string();\n'
    '                if yol.is_empty() {\n'
    '                    self.stok_durum = "Önce bir fotoğraf yolu girin.".to_string();\n'
    '                } else {\n'
    '                    let (tx, rx) = channel();\n'
    '                    self.stok_islem = Some(rx);\n'
    '                    self.stok_mesgul = true;\n'
    '                    self.stok_durum = "Fotoğraf hareketli döngüye çevriliyor…".to_string();\n'
    '                    thread::spawn(move || {\n'
    '                        match stok_video::foto_ken_burns(&grup, &yol) {\n'
    '                            Ok(hazir) => {\n'
    '                                let _ = tx.send(StokOlay::Hazir(hazir));\n'
    '                            }\n'
    '                            Err(hata) => {\n'
    '                                let _ = tx.send(StokOlay::Hata(hata));\n'
    '                            }\n'
    '                        }\n'
    '                        let _ = tx.send(StokOlay::Bitti);\n'
    '                    });\n'
    '                }\n'
    '            }\n'
    '        });\n\n'
    '        // Sürükle-bırak: pencereye bırakılan dosya yol alanına düşer.',
)
yaz(
    'src/main.rs',
    '        self.sektor_medya_olaylari();\n        bolum_etiketi(ui, "SEKTÖREL MEDYA TARAMASI");',
    '        self.sektor_medya_olaylari();\n'
    '        bolum_etiketi(ui, "SEKTÖREL MEDYA");\n'
    '        if !sektor_medya::DIS_KAYNAK_ACIK {\n'
    '            ui.label(\n'
    '                egui::RichText::new(\n'
    '                    "Dış medya kapalı: arka planlar, dokular ve geçişler işletme tohumundan kod tabanlı üretiliyor. İndirme/arama yapılmaz.",\n'
    '                )\n'
    '                .size(9.6)\n'
    '                .color(DIM),\n'
    '            );\n'
    '            return;\n'
    '        }',
)

print('tamam:', len(yapilan), 'değişiklik')
