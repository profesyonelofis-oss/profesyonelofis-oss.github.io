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

# ---------------------------------------------------------------------------
# 1) main.rs — modül + alanlar
# ---------------------------------------------------------------------------
degistir(
    'src/main.rs',
    "mod isletme_linki;",
    "mod isletme_linki;\nmod onaylanan;",
    "main.rs mod onaylanan",
)
degistir(
    'src/main.rs',
    "    /// [2] sekmesinin durum satırı: çözülüyor / çözüldü / hata.\n    seo_durum: String,",
    "    /// [2] sekmesinin durum satırı: çözülüyor / çözüldü / hata.\n    seo_durum: String,\n    /// En son çözümü başlatılan link: aynı link için tekrar istek atılmaz.\n    seo_son_link: String,\n    /// Link alanındaki son değişikliğin anı (yazma bitene kadar bekleme).\n    seo_son_degisim: f64,",
    "main.rs seo_son_link alanları",
)
degistir(
    'src/main.rs',
    "            seo_durum: String::new(),",
    "            seo_durum: String::new(),\n            seo_son_link: String::new(),\n            seo_son_degisim: 0.0,",
    "main.rs seo_son_link init",
)

# ---------------------------------------------------------------------------
# 2) katmanlar.rs — link girildiği an otomatik çözüm + SEO tetiği
# ---------------------------------------------------------------------------
p = 'src/katmanlar.rs'
s = oku(p)

# 2a. Metin alanı: değişiklik anını kaydet
eski = """                    let genislik = (ui.available_width() - 270.0).max(220.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.seo_link)
                            .hint_text("https://www.google.com/maps/place/...")
                            .desired_width(genislik),
                    );"""
yeni = """                    let genislik = (ui.available_width() - 270.0).max(220.0);
                    let alan = ui.add(
                        egui::TextEdit::singleline(&mut self.seo_link)
                            .hint_text("Google İşletme linkini yapıştırın")
                            .desired_width(genislik),
                    );
                    if alan.changed() {
                        self.seo_son_degisim = ctx.input(|girdi| girdi.time);
                    }"""
if eski not in s:
    sys.exit("BULUNAMADI: seo link metin alani")
s = s.replace(eski, yeni, 1)

# 2b. KONUMU ÇÖZ düğmesi: her zaman arka plan yolunu kullan
bas = s.find('                    if birincil_buton(ui, "KONUMU ÇÖZ"')
son = s.find('                    if ikincil_buton(ui, "SEO MOTORU"')
if bas == -1 or son == -1 or son < bas:
    sys.exit("BULUNAMADI: KONUMU COZ blogu")
YENI_BLOK = '''                    if birincil_buton(ui, "KONUMU ÇÖZ", true, Some(egui::vec2(112.0, 30.0)))
                        .clicked()
                    {
                        self.seo_son_link.clear();
                        self.seo_son_degisim = 0.0;
                        self.seo_linki_otomatik(ctx);
                    }
'''
s = s[:bas] + YENI_BLOK + s[son:]

# 2c. Panel gövdesinde otomatik tetiği çağır
kanca = "                ui.add_space(12.0);\n                match self.seo_konum.clone() {"
if kanca not in s:
    sys.exit("BULUNAMADI: seo_alani harita blogu")
s = s.replace(kanca, "                self.seo_linki_otomatik(ctx);\n" + kanca, 1)

# 2d. Arka plan sonucu gelince SEO motoru kendiliğinden çalışsın
eski = """                Ok(Ok(konum)) => {
                    let satir = format!("[✓] Konum çözüldü: {} · {}", konum.ad, konum.koordinat());
                    self.logs.push(satir.clone());
                    self.seo_durum = satir;
                    self.seo_konum = Some(konum);
                }"""
yeni = "                Ok(Ok(konum)) => self.seo_konum_geldi(konum),"
if eski not in s:
    sys.exit("BULUNAMADI: kanal Ok blogu")
s = s.replace(eski, yeni, 1)

# 2e. [3] medya bağları listesi
eski = '                alan("medya", &format!("{medya} dosya"), 40);'
yeni = '''                alan("medya", &format!("{medya} dosya"), 40);
                let medya_yollari: Vec<String> = site
                    .get("medya")
                    .and_then(|m| m.as_array())
                    .map(|m| {
                        m.iter()
                            .filter_map(|kayit| {
                                kayit
                                    .get("yol")
                                    .and_then(|yol| yol.as_str())
                                    .map(|yol| yol.to_string())
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                // Medya bağları SSR sunucusundan 200 OK ile servis edilir.
                for yol in medya_yollari.iter().take(4) {
                    ui.label(
                        egui::RichText::new(format!("GET /{yol}"))
                            .size(9.0)
                            .monospace()
                            .color(SOLUK),
                    );
                }'''
if eski not in s:
    sys.exit("BULUNAMADI: medya alani")
s = s.replace(eski, yeni, 1)

# 2f. Onay: kalıcı deftere yaz
eski = """                        if !self.seo_onaylanan.contains(&slug) {
                            self.seo_onaylanan.push(slug.clone());
                        }
                        self.logs.push(format!(
                            "[✓] {slug} SEO önizlemesinden onaylandı; Gelen Kutusu'na aktarıldı."
                        ));
                        self.katman = Katman::GelenKutusu;"""
yeni = """                        let kayit = crate::onaylanan::Kayit {
                            slug: slug.clone(),
                            ad: metin_alani("ad"),
                            sema: metin_alani("sema"),
                            baslik: metin_alani("baslik"),
                            zaman: chrono::Local::now()
                                .format("%Y-%m-%d %H:%M")
                                .to_string(),
                        };
                        match crate::onaylanan::ekle(kayit) {
                            Ok(sayi) => self.logs.push(format!(
                                "[✓] {slug} onaylandı · Gelen Kutusu'nda {sayi} onaylı işletme."
                            )),
                            Err(hata) => self.logs.push(format!(
                                "[!] Onay deftere yazılamadı: {hata}"
                            )),
                        }
                        if !self.seo_onaylanan.contains(&slug) {
                            self.seo_onaylanan.push(slug.clone());
                        }
                        self.katman = Katman::GelenKutusu;"""
if eski not in s:
    sys.exit("BULUNAMADI: onay blogu")
s = s.replace(eski, yeni, 1)

# 2g. Onay düğmesinin yanına tarayıcı önizlemesi
eski = """                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("onaylanan: {}", self.seo_onaylanan.len()))
                            .size(10.0)
                            .color(DIM),
                    );"""
yeni = """                    if ikincil_buton(ui, "TARAYICIDA AÇ", true, Some(egui::vec2(150.0, 34.0)))
                        .clicked()
                    {
                        let adres = format!("http://127.0.0.1:8090/s/{slug}");
                        match std::process::Command::new("cmd")
                            .args(["/C", "start", "", &adres])
                            .spawn()
                        {
                            Ok(_) => self.logs.push(format!("[i] Tarayıcıda açıldı: {adres}")),
                            Err(hata) => {
                                self.logs.push(format!("[!] Tarayıcı açılamadı: {hata}"))
                            }
                        }
                    }
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("onaylanan: {}", self.seo_onaylanan.len()))
                            .size(10.0)
                            .color(DIM),
                    );"""
if eski not in s:
    sys.exit("BULUNAMADI: onay sayaci")
s = s.replace(eski, yeni, 1)

# 2h. og:video de kontrol edilsin
eski = '        "application/ld+json",\n        "loading=\\"lazy\\"",'
yeni = '        "application/ld+json",\n        "property=\\"og:video\\"",\n        "loading=\\"lazy\\"",'
if eski not in s:
    sys.exit("BULUNAMADI: ssr_ozet etiket listesi")
s = s.replace(eski, yeni, 1)

# 2i. Yeni yardımcılar: otomatik çözüm + konum geldi
EK = '''
impl LeadApp {
    /// Link yapıştırıldığı anda çözümü başlatır: ağ isteği arka planda
    /// çalışır, sonuç gelince SEO motoru kendiliğinden tetiklenir.
    fn seo_linki_otomatik(&mut self, ctx: &egui::Context) {
        let link = self.seo_link.trim().to_string();
        if link.is_empty() || link == self.seo_son_link || self.seo_kanal.is_some() {
            return;
        }
        // Yazma bitene kadar bekle (600 ms): her tuşta istek atılmaz.
        let simdi = ctx.input(|girdi| girdi.time);
        if simdi - self.seo_son_degisim < 0.6 {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
            return;
        }
        self.seo_son_link = link.clone();
        let (gonderen, alan) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = gonderen.send(crate::isletme_linki::konum_coz(&link));
        });
        self.seo_kanal = Some(alan);
        self.seo_durum = "Link çözülüyor… (arka planda)".to_string();
        ctx.request_repaint();
    }

    /// Arka plandan konum geldi: harita işareti tazelenir ve SEO motoru
    /// (meta + JSON-LD + sayfa + sitemap) kendiliğinden çalışır.
    fn seo_konum_geldi(&mut self, konum: crate::isletme_linki::Konum) {
        let satir = format!("[✓] Konum çözüldü: {} · {}", konum.ad, konum.koordinat());
        self.logs.push(satir);
        self.seo_konum = Some(konum);
        self.seo_isletmesini_ekle();
    }
}
'''
s = s.rstrip() + "\n" + EK
yaz(p, s)
print("degisti: katmanlar.rs otomatik link akisi")

# ---------------------------------------------------------------------------
# 3) katman34.rs — [4 Gelen] ekranında onaylı işletmeler bölümü
# ---------------------------------------------------------------------------
p = 'src/katman34.rs'
s = oku(p)
eski = """                    pill(ui, &format!("{sicak} sıcak"), if sicak > 0 { YESIL } else { DIM });
                });
                ui.add_space(8.0);"""
yeni = """                    pill(ui, &format!("{sicak} sıcak"), if sicak > 0 { YESIL } else { DIM });
                });
                ui.add_space(8.0);
                // [3] SEO önizlemesinde onaylanan işletmeler: kalıcı defter.
                let onayli = crate::onaylanan::yukle();
                if !onayli.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("SEO ONAYLI · {}", onayli.len()))
                            .size(9.0)
                            .color(KEHRIBAR),
                    );
                    for kayit in onayli.iter().take(4) {
                        ui.label(
                            egui::RichText::new(format!("· {}", kayit.ad))
                                .size(9.5)
                                .color(YESIL),
                        );
                    }
                    ui.separator();
                }"""
if eski not in s:
    sys.exit("BULUNAMADI: gelen ekrani pill blogu")
s = s.replace(eski, yeni, 1)
# Bölüm başlığı artık 4 katmanlı akışa göre
s = s.replace('bolum_etiketi(ui, "KATMAN 5 · GELEN KUTUSU");', 'bolum_etiketi(ui, "4 · GELEN KUTUSU");', 1)
yaz(p, s)
print("degisti: katman34.rs gelen ekrani")
print("TAMAM")
