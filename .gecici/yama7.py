import re
import sys

def oku(p):
    return open(p, encoding='utf-8').read()

def yaz(p, s):
    open(p, 'w', encoding='utf-8', newline='\n').write(s)

def degistir(p, eski, yeni, ad):
    s = oku(p)
    if eski not in s:
        sys.exit("BULUNAMADI: " + ad)
    yaz(p, s.replace(eski, yeni, 1))
    print("degisti:", ad)

def test_sil(p, ad):
    s = oku(p)
    i = s.find("fn " + ad + "(")
    if i < 0:
        print("test yok:", ad)
        return
    bas = s.rfind("    #[test]", 0, i)
    if bas < 0:
        bas = s.rfind("#[test]", 0, i)
    sonraki = s.find("    #[test]", i)
    if sonraki < 0:
        sonraki = s.find("}", i)
    yaz(p, s[:bas] + s[sonraki:])
    print("test silindi:", ad)

# ---------------------------------------------------------------- 1) main.rs
p = 'src/main.rs'
s = oku(p)

for ad in ["harita", "isletme_linki", "onaylanan", "profil", "seo_motoru", "teshis", "web_veri"]:
    s = re.sub(r"^mod " + ad + r";\n", "", s, count=1, flags=re.M)
if "seo::" not in s:
    s = re.sub(r"^mod seo;\n", "", s, count=1, flags=re.M)

# Katman: yalnız iki sekme, rakamsız
desen = re.compile(
    r"#\[derive\(Clone, Copy, PartialEq, Eq, Debug\)\]\nenum Katman \{.*?\n\}\n\nimpl Katman \{.*?\n\}\n",
    re.S,
)
yeni_enum = '''#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Katman {
    MusteriBulma,
    WhatsApp,
}

impl Katman {
    const HEPSI: [Katman; 2] = [Katman::MusteriBulma, Katman::WhatsApp];

    /// Sekme başlığı: rakam yok, yalnız çalışma alanının adı.
    fn etiket(self) -> &'static str {
        match self {
            Katman::MusteriBulma => "Müşteri Bulma",
            Katman::WhatsApp => "WhatsApp",
        }
    }

    fn kisa(self) -> &'static str {
        self.etiket()
    }
}
'''
s, sayi = desen.subn(yeni_enum, s, count=1)
if sayi != 1:
    sys.exit("Katman enum degistirilemedi")

# alan tanımları
alanlar = '''    /// [2] SEO çalışma alanına yapıştırılan Google İşletme linki.
    seo_link: String,
    /// Linkten çözülen koordinat (harita işareti).
    seo_konum: Option<crate::isletme_linki::Konum>,
    /// [3] önizlemede onaylanıp [4] Gelen'e aktarılan sayfa slug'ları.
    seo_onaylanan: Vec<String>,
    /// Paylaşım linki arka planda çözülürken açık kalan kanal.
    seo_kanal: Option<std::sync::mpsc::Receiver<Result<crate::isletme_linki::Isletme, String>>>,
    /// [2] sekmesinin durum satırı: çözülüyor / çözüldü / hata.
    seo_durum: String,
    /// En son çözümü başlatılan link: aynı link için tekrar istek atılmaz.
    seo_son_link: String,
    /// Link alanındaki son değişikliğin anı (yazma bitene kadar bekleme).
    seo_son_degisim: f64,
    /// Google Haritalar profil kaydı (adres, telefon, saatler, yorum).
    seo_kayit: Option<crate::google::BizKayit>,
    /// [2]/[3] için Local SEO denetimi (kategori, NAP, koordinat, ...).
    seo_denetim: Option<crate::profil::Denetim>,
    /// Sağdaki gerçek OpenStreetMap haritası (karo önbelleği + pin).
    harita: crate::harita::Harita,
    /// [3] SSR teşhisi: (slug, rapor) — aynı sayfa için tekrar çekilmez.
    teshis: Option<(String, crate::teshis::Rapor)>,
'''
if alanlar not in s:
    sys.exit("seo alan blogu degisti")
s = s.replace(alanlar, "", 1)

inits = '''            seo_link: String::new(),
            seo_konum: None,
            seo_onaylanan: Vec::new(),
            seo_kanal: None,
            seo_durum: String::new(),
            seo_son_link: String::new(),
            seo_son_degisim: 0.0,
            seo_kayit: None,
            seo_denetim: None,
            harita: crate::harita::Harita::default(),
            teshis: None,
'''
if inits not in s:
    sys.exit("seo init blogu degisti")
s = s.replace(inits, "", 1)

s = s.replace(
    "        // Arka plan link çözümü sekmelerden bağımsız işlenir.\n        self.seo_kanali_isle(ctx);\n",
    "",
    1,
)
s = s.replace(
    "        // Veri köprüsü katmandan bağımsız: Keşif'teyken de SEO sunucusunun\n        // okuduğu web/veri.json güncel kalsın (içerik aynıysa yazılmaz).\n        self.web_verisini_yaz();\n",
    "",
    1,
)
s = s.replace("Katman::Kesif", "Katman::MusteriBulma")
s = s.replace("Katman::GelenKutusu", "Katman::WhatsApp")
s = s.replace("Nexus Enterprise - Keşif, SEO Çalışma Alanı ve Önizleme", "Nexus Enterprise - Müşteri Bulma ve WhatsApp")
yaz(p, s)
print("main.rs temel duzenlemeler tamam")

# SEO testleri
for ad in ["gercek_tarama_seo_sitemap", "share_linki_uctan_uca"]:
    test_sil(p, ad)

# ------------------------------------------------------------- 2) katmanlar.rs
p = 'src/katmanlar.rs'
s = oku(p)

# 2a) SEO motoru + web/veri.json yazımı
desen = re.compile(
    r"impl LeadApp \{\n    /// Onaylı işletmeleri ve seçili medyayı SEO sunucusunun okuduğu.*?(?=    /// Katman 2–5'in ekranı\.)",
    re.S,
)
s, sayi = desen.subn("impl LeadApp {\n", s, count=1)
if sayi != 1:
    sys.exit("seo motoru blogu silinemedi")

# 2b) katman_ekrani: iki sekme
eski = re.compile(r"    pub\(crate\) fn katman_ekrani\(&mut self, ctx: &egui::Context, gonderen: &str\) \{.*?\n        let _ = gonderen;\n    \}", re.S)
yeni = '''    pub(crate) fn katman_ekrani(&mut self, ctx: &egui::Context, gonderen: &str) {
        match self.katman {
            Katman::MusteriBulma => {}
            Katman::WhatsApp => self.gelen_ekrani(ctx),
        }
        let _ = gonderen;
    }'''
s, sayi = eski.subn(yeni, s, count=1)
if sayi != 1:
    sys.exit("katman_ekrani degistirilemedi")

# 2c) SEO ekranlarını dosya sonundan kes
i = s.find("impl LeadApp {\n    /// Katman 2 — Google İşletme linki")
if i < 0:
    sys.exit("SEO ekran blogu bulunamadi")
s = s[:i].rstrip() + "\n"
yaz(p, s)
print("katmanlar.rs SEO ekranlari kaldirildi")

# 2d) üst bar: rakamsız etiket, SEO motoru düğmesi yok
degistir(
    p,
    'let metin = format!("{} {}{rozet}", katman.no(), katman.kisa());',
    'let metin = format!("{}{rozet}", katman.kisa());',
    "ust bar etiket",
)
degistir(
    p,
    '''                        self.logs.push(format!(
                            "[i] Katman {}: {} · {}",
                            katman.no(),
                            katman.etiket(),
                            "çalışma alanı değişti"
                        ));''',
    '''                        self.logs.push(format!(
                            "[i] {} · çalışma alanı değişti",
                            katman.etiket()
                        ));''',
    "katman gunlugu",
)
degistir(
    p,
    '''                        if birincil_buton(
                            ui,
                            "SEO MOTORU",
                            true,
                            Some(egui::vec2(126.0, 26.0)),
                        )
                        .on_hover_text(
                            "Seçili işletmelerin sektör medyasını bağlar; 60/160 karakterlik başlık-açıklama, canonical, Open Graph, JSON-LD ve sitemap üretimini anında yayına yazar",
                        )
                        .clicked()
                        {
                            self.seo_calistir();
                        }
                        ui.add_space(8.0);''',
    "",
    "SEO MOTORU dugmesi",
)
s = oku(p)
s = s.replace("katman == Katman::GelenKutusu", "katman == Katman::WhatsApp")
# 2e) medya işleyicisindeki otomatik SEO çağrısı
s = s.replace(
    "                                // doğrudan SEO motorunun arşivine düşer.\n                                self.seo_calistir();\n",
    "",
    1,
)
yaz(p, s)
print("katmanlar.rs ust bar temizlendi")

# ------------------------------------------------------------- 3) katman34.rs
p = 'src/katman34.rs'
s = oku(p)
desen = re.compile(
    r"                // \[3\] SEO önizlemesinde onaylanan işletmeler: kalıcı defter\.\n                let onayli = crate::onaylanan::yukle\(\);.*?                    ui\.separator\(\);\n                \}\n",
    re.S,
)
s, sayi = desen.subn("", s, count=1)
if sayi != 1:
    sys.exit("onayli liste blogu silinemedi")
s = s.replace('bolum_etiketi(ui, "4 · GELEN KUTUSU");', 'bolum_etiketi(ui, "WHATSAPP");', 1)
yaz(p, s)
print("katman34.rs onayli liste kaldirildi, baslik WHATSAPP")

# ------------------------------------------------------------- 4) seo.rs
if "seo::" not in oku('src/main.rs'):
    print("seo.rs bagimsiz: silinecek")
    yaz('src/seo_baglanti_yok.txt', "")
else:
    print("UYARI: seo:: referansi var, seo.rs korunuyor")
print("TAMAM")
