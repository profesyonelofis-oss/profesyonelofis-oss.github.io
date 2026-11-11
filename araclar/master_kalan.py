"""Kalan yamalar: main.rs %50 harita + odak, katmanlar hot-reload, google bulut_temizle."""
import re
import sys

sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def oku(yol):
    return open(yol, encoding="utf-8").read()


def yaz(yol, metin):
    open(yol, "w", encoding="utf-8", newline="\n").write(metin)


def re_degistir(metin, desen, yeni, ad, sayi=1):
    yeni_metin, adet = re.subn(desen, lambda _: yeni, metin, count=sayi, flags=re.S)
    assert adet == sayi, f"{ad}: {adet} eslesme"
    return yeni_metin


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
    raise AssertionError("blok sonu yok")


# ===========================================================================
# main.rs
# ===========================================================================
m = oku("src/main.rs")

m = re_degistir(
    m,
    r'let harita_genisligi = .*?;\n\s*egui::SidePanel::right\("kesif_harita"\)\n\s*\.resizable\(true\)\n\s*\.default_width\(harita_genisligi\)\n\s*\.min_width\([0-9.]+\)\n\s*\.max_width\([0-9.]+\)',
    """// Piksel sabit sağ panel: ekran genişliğinin tam yarısı.
        let harita_genisligi = (ctx.screen_rect().width() * 0.5).max(380.0);
        egui::SidePanel::right("kesif_harita")
            .resizable(false)
            .exact_width(harita_genisligi)""",
    "harita %50 sabit",
)

m = re_degistir(
    m,
    r'let sol_genislik = \(ctx\.screen_rect\(\)\.width\(\) \* [0-9.]+\)\.clamp\([0-9.]+, [0-9.]+\);',
    "let sol_genislik = (ctx.screen_rect().width() * 0.22).clamp(260.0, 330.0);",
    "sol sütun",
)

# Harita odağı alanı.
assert m.count("    vurgu_kart: Option<usize>,") == 1
m = m.replace(
    "    vurgu_kart: Option<usize>,",
    "    vurgu_kart: Option<usize>,\n"
    "    /// Listeden seçilen işletme: harita bu pine yumuşak geçiş yapar.\n"
    "    harita_odak: Option<usize>,",
    1,
)
assert m.count("vurgu_kart: None,") == 1
m = m.replace("vurgu_kart: None,", "vurgu_kart: None,\n            harita_odak: None,", 1)

# Liste tıklaması haritayı odaklar.
assert m.count("let mut onizleme_idx: Option<usize> = None;") == 1
m = m.replace(
    "let mut onizleme_idx: Option<usize> = None;",
    "let mut onizleme_idx: Option<usize> = None;\n        let mut odak_idx: Option<usize> = None;",
    1,
)

i = m.find("yeni_detay = if acik_detay == Some(idx)")
assert i > 0, "yeni_detay ataması yok"
geri = m.rfind("if detay_iste {", 0, i)
assert geri > 0, "detay_iste koşulu yok"
j = blok_sonu(m, geri)
m = (
    m[:j]
    + "\n                                if detay_iste && acik_detay != Some(idx) {\n"
    + "                                    // Listeden seçilen işletme haritada odaklanır.\n"
    + "                                    odak_idx = Some(idx);\n"
    + "                                }"
    + m[j:]
)

if "if let Some(idx) = onizleme_idx {" in m:
    m = m.replace(
        "                if let Some(idx) = onizleme_idx {",
        "                if let Some(idx) = odak_idx {\n"
        "                    self.harita_odak = Some(idx);\n"
        "                }\n"
        "                if let Some(idx) = onizleme_idx {",
        1,
    )
else:
    raise AssertionError("onizleme_idx aktarımı yok")

yaz("src/main.rs", m)
print("main.rs: %50 harita + harita_odak")

# ===========================================================================
# katmanlar.rs
# ===========================================================================
k = oku("src/katmanlar.rs")

k = re_degistir(
    k,
    r'( *)(let \(noktalar, konumsuz\) = harita::noktalar\(&self\.leads, self\.detay_idx\);)',
    """let (noktalar, konumsuz) = harita::noktalar(&self.leads, self.detay_idx);
        // Listeden seçilen işletme haritada yumuşak geçişle odaklanır.
        if let Some(idx) = self.harita_odak.take() {
            if let Some(lead) = self.leads.get(idx) {
                if let Some(nokta) = noktalar.iter().find(|nokta| nokta.ad == lead.name) {
                    self.harita.odaklan_nokta(nokta);
                }
            }
        }""",
    "kesif_haritasi odak",
)

k = re_degistir(
    k,
    r'match anahtar::Kasa::kaydet\(&girdi\) \{.*?Err\(hata\) => self\.api_mesaj = format!\("\[!\] \{hata\}"\),\s*\}',
    """match anahtar::Kasa::kaydet(&girdi) {
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

k = re_degistir(
    k,
    r'match anahtar::Kasa::sil\(\) \{\s*Ok\(\(\)\) => self\.api_mesaj = "Anahtar silindi\."\.to_string\(\),\s*Err\(hata\) => self\.api_mesaj = format!\("\[!\] \{hata\}"\),\s*\}',
    """match anahtar::Kasa::sil() {
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
# google.rs
# ===========================================================================
g = oku("src/google.rs")
assert "pub fn bulut_temizle" not in g
iface = "/// Senkron hatasını panelde gösterir ve `bulut_senkron.log` dosyasına yazar."
assert iface in g, "bulut_hata_yaz doc yok"
g = g.replace(
    iface,
    """/// Anahtar/proje değiştiğinde bulut sayaç önbelleğini sıfırlar.
/// Paneldeki eski oturum verisi yeni anahtarla karışmasın diye çağrılır.
pub fn bulut_temizle() {
    let kilit = kota();
    let mut sayac = kilit.lock().unwrap_or_else(|zehir| zehir.into_inner());
    sayac.bulut_gunluk = None;
    sayac.bulut_aylik = None;
    sayac.bulut_places = None;
    sayac.bulut_geocode = None;
    sayac.bulut_zaman = None;
    sayac.bulut_hata = None;
    let _ = sayac.kaydet_yoldan(KOTA_DOSYASI);
}

""" + iface,
    1,
)
yaz("src/google.rs", g)
print("google.rs: bulut_temizle")
