"""Master architecture patch: MASKİ focus, fixed 50% map, hot-reload key."""
import re
import sys

sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def oku(yol):
    return open(yol, encoding="utf-8").read()


def yaz(yol, metin):
    open(yol, "w", encoding="utf-8", newline="\n").write(metin)


def degistir(metin, eski, yeni, ad):
    assert metin.count(eski) == 1, f"{ad}: {metin.count(eski)} eslesme"
    return metin.replace(eski, yeni, 1)


def blok_sonu(metin, bas):
    k = metin.find("{", bas)
    assert k > 0
    d = 0
    while k < len(metin):
        if metin[k] == "{":
            d += 1
        elif metin[k] == "}":
            d -= 1
            if d == 0:
                return k + 1
        k += 1
    raise AssertionError("blok sonu bulunamadi")


# ===========================================================================
# harita.rs
# ===========================================================================
h = oku("src/harita.rs")

# 1) MASKİ merkezi ve sokak ölçeği.
h = degistir(
    h,
    "/// Varsayılan merkez: Manisa · Şehzadeler (kullanıcının evi).\npub const EV: (f64, f64) = (38.6115, 27.4258);",
    "/// Varsayılan odak: Manisa · Şehzadeler · Yarhasanlar, 2324. Sokak\n"
    "/// (MASKİ Genel Müdürlüğü). Harita açılışta bu noktaya kilitlenir.\n"
    "pub const EV: (f64, f64) = (38.6146, 27.4273);",
    "MASKİ merkezi",
)
h = degistir(h, "const VARSAYILAN_ZOOM: u8 = 16;", "const VARSAYILAN_ZOOM: u8 = 17;", "varsayılan zoom")
h = degistir(
    h,
    "/// Google Static Maps dosemesi: 640x640 px, tek istekte gorunumu kaplar.",
    "/// Yumuşak kamera geçişinin süresi (saniye).\n"
    "const ANIMASYON_SN: f32 = 0.45;\n"
    "/// Google Static Maps dosemesi: 640x640 px, tek istekte gorunumu kaplar.",
    "animasyon sabiti",
)

# 2) Pin bilgi kartı için adres/telefon satırı.
h = degistir(
    h,
    "pub struct Nokta {\n    pub ad: String,\n    pub etiket: String,\n    pub konum: (f64, f64),\n    pub secili: bool,\n}",
    "pub struct Nokta {\n"
    "    pub ad: String,\n"
    "    pub etiket: String,\n"
    "    /// Bilgi kartında gösterilen telefon · adres satırı.\n"
    "    pub alt: String,\n"
    "    pub konum: (f64, f64),\n"
    "    pub secili: bool,\n"
    "}",
    "Nokta.alt",
)
h = degistir(
    h,
    "        liste.push(Nokta {\n            ad: lead.name.clone(),\n            etiket: lead.category.clone(),\n            konum,\n            secili: secili == Some(idx),\n        });",
    "        let alt = match (lead.phone.trim(), lead.address.trim()) {\n"
    "            (\"\", \"\") => String::new(),\n"
    "            (\"\", adres) => adres.to_string(),\n"
    "            (telefon, \"\") => telefon.to_string(),\n"
    "            (telefon, adres) => format!(\"{telefon} · {adres}\"),\n"
    "        };\n"
    "        liste.push(Nokta {\n"
    "            ad: lead.name.clone(),\n"
    "            etiket: lead.category.clone(),\n"
    "            alt,\n"
    "            konum,\n"
    "            secili: secili == Some(idx),\n"
    "        });",
    "noktalar alt satırı",
)

# 3) Animasyon durumu.
h = degistir(
    h,
    "/// Yuklenmis Google Maps dosemesi ve hangi gorunume ait oldugu.",
    "/// Yumuşak kamera geçişi: başlangıç/hedef merkez ve zoom.\n"
    "struct Animasyon {\n"
    "    bas_merkez: (f64, f64),\n"
    "    hedef_merkez: (f64, f64),\n"
    "    bas_zoom: f64,\n"
    "    hedef_zoom: f64,\n"
    "    basladi: Instant,\n"
    "}\n"
    "\n"
    "/// Yuklenmis Google Maps dosemesi ve hangi gorunume ait oldugu.",
    "Animasyon struct",
)
h = degistir(
    h,
    "    doseme_sayaci: u32,\n    hata: Option<String>,\n}",
    "    doseme_sayaci: u32,\n    hata: Option<String>,\n    anim: Option<Animasyon>,\n}",
    "Harita.anim",
)
h = degistir(
    h,
    "            doseme_sayaci: 0,\n            hata: None,\n        }",
    "            doseme_sayaci: 0,\n            hata: None,\n            anim: None,\n        }",
    "Harita.anim varsayılan",
)

# 4) eve_don + anahtar yenileme + yumuşak odak.
h = degistir(
    h,
    "    pub fn eve_don(&mut self) {\n        self.merkez = EV;\n        self.zoom = VARSAYILAN_ZOOM;\n    }",
    "    pub fn eve_don(&mut self) {\n        self.hedefe_animasyon(EV, VARSAYILAN_ZOOM);\n    }\n"
    "\n"
    "    /// API anahtarı değiştiğinde çağrılır: anahtar, döşeme ve sayaçlar\n"
    "    /// sıfırlanır; sonraki karede yeni anahtar kasadan okunur.\n"
    "    pub fn anahtar_yenile(&mut self) {\n"
    "        self.anahtar = None;\n"
    "        self.doku = None;\n"
    "        self.istek = None;\n"
    "        self.hata = None;\n"
    "        self.deneme = 0;\n"
    "        self.son_istek = None;\n"
    "        self.doseme_sayaci = 0;\n"
    "    }\n"
    "\n"
    "    /// Tek bir işletmeye yumuşak geçiş (listeden seçim).\n"
    "    pub fn odaklan_nokta(&mut self, nokta: &Nokta) {\n"
    "        self.hedefe_animasyon(nokta.konum, 17);\n"
    "    }\n"
    "\n"
    "    fn hedefe_animasyon(&mut self, merkez: (f64, f64), zoom: u8) {\n"
    "        self.anim = Some(Animasyon {\n"
    "            bas_merkez: self.merkez,\n"
    "            hedef_merkez: merkez,\n"
    "            bas_zoom: self.zoom as f64,\n"
    "            hedef_zoom: zoom.clamp(MIN_ZOOM, MAX_ZOOM) as f64,\n"
    "            basladi: Instant::now(),\n"
    "        });\n"
    "    }\n"
    "\n"
    "    /// Kamerayı hedefe doğru yumuşakça kaydırır.\n"
    "    fn animasyon_adim(&mut self, ctx: &egui::Context) {\n"
    "        let Some(anim) = &self.anim else {\n"
    "            return;\n"
    "        };\n"
    "        let t = (anim.basladi.elapsed().as_secs_f32() / ANIMASYON_SN).clamp(0.0, 1.0);\n"
    "        let e = 1.0 - (1.0 - t).powi(3);\n"
    "        self.merkez = (\n"
    "            anim.bas_merkez.0 + (anim.hedef_merkez.0 - anim.bas_merkez.0) * e as f64,\n"
    "            anim.bas_merkez.1 + (anim.hedef_merkez.1 - anim.bas_merkez.1) * e as f64,\n"
    "        );\n"
    "        self.zoom = (anim.bas_zoom + (anim.hedef_zoom - anim.bas_zoom) * e as f64) as u8;\n"
    "        if t >= 1.0 {\n"
    "            self.merkez = anim.hedef_merkez;\n"
    "            self.zoom = anim.hedef_zoom as u8;\n"
    "            self.anim = None;\n"
    "        }\n"
    "        ctx.request_repaint();\n"
    "    }",
    "eve_don + anahtar_yenile + animasyon",
)

# 5) sonuclara_odaklan: hedefi hesapla, animasyonu başlat.
bas = h.find("    pub fn sonuclara_odaklan(&mut self, noktalar: &[Nokta]) {")
assert bas > 0
son = h.find("    /// İmlecin altındaki noktayı sabit tutarak yakınlaştırır.", bas)
assert son > bas
h = h[:bas] + """    /// Yeni sonuçlarda kamera pinlerin üzerine yumuşak geçişle yaklaşır.
    pub fn sonuclara_odaklan(&mut self, noktalar: &[Nokta]) {
        let Some(ilk) = noktalar.first() else {
            return;
        };
        let (mut guney, mut kuzey) = (ilk.konum.0, ilk.konum.0);
        let (mut bati, mut dogu) = (ilk.konum.1, ilk.konum.1);
        for nokta in noktalar {
            guney = guney.min(nokta.konum.0);
            kuzey = kuzey.max(nokta.konum.0);
            bati = bati.min(nokta.konum.1);
            dogu = dogu.max(nokta.konum.1);
        }
        let merkez = ((guney + kuzey) / 2.0, (bati + dogu) / 2.0);
        let aciklik = (kuzey - guney).max(dogu - bati);
        let zoom = if noktalar.len() == 1 {
            17
        } else if aciklik <= 0.02 {
            14
        } else if aciklik <= 0.1 {
            12
        } else if aciklik <= 0.5 {
            10
        } else {
            8
        };
        self.hedefe_animasyon(merkez, zoom);
    }

""" + h[son:]

# 6) Kullanıcı müdahalesi animasyonu iptal eder; animasyon her kare ilerler.
h = degistir(
    h,
    "        if yanit.dragged_by(egui::PointerButton::Primary) {\n            let delta = yanit.drag_delta();",
    "        if yanit.dragged() {\n            self.anim = None;\n        }\n"
    "        self.animasyon_adim(ui.ctx());\n"
    "        if yanit.dragged_by(egui::PointerButton::Primary) {\n            let delta = yanit.drag_delta();",
    "animasyon ilerletme",
)

# 7) Seçili pin için Google tarzı bilgi kartı (ad + telefon/adres).
desen = re.compile(
    r"( *)(p\.galley\(kutu\.min \+ egui::vec2\([0-9.]+, [0-9.]+\), yazi, VEKTOR_YAZI\);)"
)
assert desen.search(h), "galley satırı yok"
h = desen.sub(
    lambda e: (
        f"{e.group(1)}{e.group(2)}\n"
        f"{e.group(1)}// Seçili pinin bilgi kartı: Google Maps info window.\n"
        f"{e.group(1)}if nokta.secili && !nokta.alt.trim().is_empty() {{\n"
        f"{e.group(1)}    let alt_yazi = p.layout_no_wrap(\n"
        f"{e.group(1)}        nokta.alt.clone(),\n"
        f"{e.group(1)}        egui::FontId::proportional(9.5),\n"
        f"{e.group(1)}        VEKTOR_SOLUK,\n"
        f"{e.group(1)}    );\n"
        f"{e.group(1)}    let alt_kutu = egui::Rect::from_min_size(\n"
        f"{e.group(1)}        kutu.left_bottom() + egui::vec2(0.0, 3.0),\n"
        f"{e.group(1)}        alt_yazi.size() + egui::vec2(12.0, 6.0),\n"
        f"{e.group(1)}    );\n"
        f"{e.group(1)}    p.rect_filled(alt_kutu, egui::Rounding::same(4.0), BEYAZ);\n"
        f"{e.group(1)}    p.rect_stroke(\n"
        f"{e.group(1)}        alt_kutu,\n"
        f"{e.group(1)}        egui::Rounding::same(4.0),\n"
        f"{e.group(1)}        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(218, 220, 224)),\n"
        f"{e.group(1)}    );\n"
        f"{e.group(1)}    p.galley(alt_kutu.min + egui::vec2(6.0, 3.0), alt_yazi, VEKTOR_SOLUK);\n"
        f"{e.group(1)}}}"
    ),
    h,
    count=1,
)
yaz("src/harita.rs", h)
print("harita.rs: MASKİ merkez, animasyon, bilgi kartı, anahtar_yenile")

# ===========================================================================
# main.rs
# ===========================================================================
m = oku("src/main.rs")

# 1) Sağ panel: piksel sabit %50, yeniden boyutlandırılamaz.
m = degistir(
    m,
    """        let harita_genisligi = (ctx.screen_rect().width() * 0.42).clamp(420.0, 880.0);
        egui::SidePanel::right("kesif_harita")
            .resizable(true)
            .default_width(harita_genisligi)
            .min_width(380.0)
            .max_width(900.0)""",
    """        // Piksel sabit sağ panel: ekran genişliğinin tam yarısı.
        let harita_genisligi = (ctx.screen_rect().width() * 0.5).max(380.0);
        egui::SidePanel::right("kesif_harita")
            .resizable(false)
            .exact_width(harita_genisligi)""",
    "harita %50 sabit",
)

# 2) Sol sütun daraltılır: orta arama sütununa yer açılır.
m = degistir(
    m,
    "let sol_genislik = (ctx.screen_rect().width() * 0.26).clamp(300.0, 390.0);",
    "let sol_genislik = (ctx.screen_rect().width() * 0.22).clamp(260.0, 330.0);",
    "sol sütun genişliği",
)

# 3) Harita odağı alanı.
m = degistir(
    m,
    "    /// Haritadan pin seçildiğinde listede kaydırılıp vurgulanacak kart.\n    vurgu_kart: Option<usize>,",
    "    /// Haritadan pin seçildiğinde listede kaydırılıp vurgulanacak kart.\n"
    "    vurgu_kart: Option<usize>,\n"
    "    /// Listeden seçilen işletme: harita bu pine yumuşak geçiş yapar.\n"
    "    harita_odak: Option<usize>,",
    "harita_odak alanı",
)
m = degistir(
    m,
    "            harita_imza: (0, None),\n            vurgu_kart: None,",
    "            harita_imza: (0, None),\n            vurgu_kart: None,\n            harita_odak: None,",
    "harita_odak başlangıcı",
)

# 4) Liste tıklaması haritayı odaklar.
m = degistir(
    m,
    "        let mut onizleme_idx: Option<usize> = None;",
    "        let mut onizleme_idx: Option<usize> = None;\n        let mut odak_idx: Option<usize> = None;",
    "odak_idx tanımı",
)
i = m.find("                                if detay_iste {")
assert i > 0, "detay_iste bloğu yok"
j = blok_sonu(m, i)
m = (
    m[:j]
    + "\n                                if detay_iste && acik_detay != Some(idx) {\n"
    + "                                    // Listeden seçilen işletme haritada odaklanır.\n"
    + "                                    odak_idx = Some(idx);\n"
    + "                                }"
    + m[j:]
)
m = degistir(
    m,
    "                if let Some(idx) = onizleme_idx {",
    "                if let Some(idx) = odak_idx {\n"
    "                    self.harita_odak = Some(idx);\n"
    "                }\n"
    "                if let Some(idx) = onizleme_idx {",
    "odak_idx aktarımı",
)
yaz("src/main.rs", m)
print("main.rs: %50 harita, harita_odak")

# ===========================================================================
# katmanlar.rs
# ===========================================================================
k = oku("src/katmanlar.rs")

# 1) Listeden seçilen pini haritada odakla.
k = degistir(
    k,
    "        let (noktalar, konumsuz) = harita::noktalar(&self.leads, self.detay_idx);",
    "        let (noktalar, konumsuz) = harita::noktalar(&self.leads, self.detay_idx);\n"
    "        // Listeden seçilen işletme haritada yumuşak geçişle odaklanır.\n"
    "        if let Some(idx) = self.harita_odak.take() {\n"
    "            if let Some(lead) = self.leads.get(idx) {\n"
    "                if let Some(nokta) = noktalar.iter().find(|nokta| nokta.ad == lead.name) {\n"
    "                    self.harita.odaklan_nokta(nokta);\n"
    "                }\n"
    "            }\n"
    "        }",
    "kesif_haritasi odak",
)

# 2) KAYDET: sıcak yeniden yükleme + oturum önbelleği + ön kontrol pingi.
k = degistir(
    k,
    """                        match anahtar::Kasa::kaydet(&girdi) {
                            Ok(()) => {
                                self.api_mesaj =
                                    "✓ Anahtar şifrelenerek kaydedildi.".to_string();
                                self.api_girdi.clear();
                            }
                            Err(hata) => self.api_mesaj = format!("[!] {hata}"),
                        }""",
    """                        match anahtar::Kasa::kaydet(&girdi) {
                            Ok(()) => {
                                // Sıcak yeniden yükleme: .exe yeniden başlamaz.
                                // Harita döşemesi, bulut sayaçları ve oturum
                                // önbelleği temizlenir; yeni anahtar anında devreye girer.
                                self.harita.anahtar_yenile();
                                google::bulut_temizle();
                                self.api_girdi.clear();
                                let proje = ayar_oku().google_proje;
                                if !proje.is_empty() && self.api_bulut.is_none() {
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    let anahtar = girdi.clone();
                                    std::thread::spawn(move || {
                                        let _ = tx.send(google::bulut_senkron(&anahtar, &proje));
                                    });
                                    self.api_bulut = Some(rx);
                                    self.api_mesaj = "✓ Anahtar kaydedildi; bağlantı sınanıyor ve bulut sayaçları yenileniyor…".to_string();
                                } else if self.api_test.is_none() {
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    let anahtar = girdi.clone();
                                    std::thread::spawn(move || {
                                        let _ = tx.send(google::anahtar_testi(&anahtar));
                                    });
                                    self.api_test = Some(rx);
                                    self.api_mesaj = "✓ Anahtar kaydedildi; bağlantı sınanıyor…".to_string();
                                } else {
                                    self.api_mesaj = "✓ Anahtar kaydedildi; harita yeni anahtarla yükleniyor.".to_string();
                                }
                            }
                            Err(hata) => self.api_mesaj = format!("[!] {hata}"),
                        }""",
    "KAYDET hot-reload",
)

# 3) ANAHTARI SİL: harita önbelleği de düşer.
k = degistir(
    k,
    """                        match anahtar::Kasa::sil() {
                            Ok(()) => self.api_mesaj = "Anahtar silindi.".to_string(),
                            Err(hata) => self.api_mesaj = format!("[!] {hata}"),
                        }""",
    """                        match anahtar::Kasa::sil() {
                            Ok(()) => {
                                self.harita.anahtar_yenile();
                                google::bulut_temizle();
                                self.api_mesaj = "Anahtar silindi.".to_string();
                            }
                            Err(hata) => self.api_mesaj = format!("[!] {hata}"),
                        }""",
    "ANAHTARI SİL",
)
yaz("src/katmanlar.rs", k)
print("katmanlar.rs: odak + hot-reload")

# ===========================================================================
# google.rs: bulut önbelleğini sıfırlama
# ===========================================================================
g = oku("src/google.rs")
g = degistir(
    g,
    "/// Senkron hatasını panelde gösterir ve `bulut_senkron.log` dosyasına yazar.",
    "/// Anahtar/proje değiştiğinde bulut sayaç önbelleğini sıfırlar.\n"
    "/// Paneldeki eski oturum verisi yeni anahtarla karışmasın diye çağrılır.\n"
    "pub fn bulut_temizle() {\n"
    "    let kilit = kota();\n"
    "    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());\n"
    "    sayac.bulut_gunluk = None;\n"
    "    sayac.bulut_aylik = None;\n"
    "    sayac.bulut_places = None;\n"
    "    sayac.bulut_geocode = None;\n"
    "    sayac.bulut_zaman = None;\n"
    "    sayac.bulut_hata = None;\n"
    "    let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);\n"
    "}\n"
    "\n"
    "/// Senkron hatasını panelde gösterir ve `bulut_senkron.log` dosyasına yazar.",
    "bulut_temizle",
)
yaz("src/google.rs", g)
print("google.rs: bulut_temizle")
