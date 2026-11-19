"""katmanlar.rs / saglik.rs / iller.rs / demo_site.rs: Google kalıntılarını sil."""

import re
from pathlib import Path


def yukle(yol: str) -> str:
    return Path(yol).read_text(encoding="utf-8")


def yaz(yol: str, s: str) -> None:
    Path(yol).write_text(s, encoding="utf-8", newline="")


def rep(s: str, eski: str, yeni: str, adet: int = 1, ad: str = "") -> str:
    c = s.count(eski)
    assert c == adet, f"{ad}: beklenen {adet}, bulunan {c}"
    return s.replace(eski, yeni)


def kes(s: str, bas: str, son: str, yeni: str = "", ad: str = "") -> str:
    assert s.count(bas) == 1, f"{ad}: başlangıç yok/çok"
    i = s.index(bas)
    j = s.index(son, i)
    return s[:i] + yeni + s[j:]


def fn_sil(s: str, ad: str, girinti: str = "") -> str:
    im = re.search(rf"(?m)^{re.escape(girinti)}(?:pub(?:\(crate\))? )?fn {re.escape(ad)}\b", s)
    assert im, f"fonksiyon yok: {ad}"
    bas = im.start()
    while True:
        onceki = s.rfind("\n", 0, bas - 1)
        if onceki < 0:
            break
        satir = s[onceki + 1 : bas].strip()
        if satir.startswith("///") or satir.startswith("#[") or satir.startswith("//"):
            bas = onceki + 1
        else:
            break
    derinlik = 0
    gordum = False
    i = im.end()
    while i < len(s):
        if s[i] == "{":
            derinlik += 1
            gordum = True
        elif s[i] == "}":
            derinlik -= 1
            if gordum and derinlik == 0:
                son = i + 1
                while son < len(s) and s[son] in "\r\n":
                    son += 1
                return s[:bas] + s[son:]
        i += 1
    raise AssertionError(f"kapanış yok: {ad}")


# --- katmanlar.rs ----------------------------------------------------------
s = yukle("src/katmanlar.rs")
s = kes(
    s,
    "    /// Katman 1'in kota şeridi:",
    "    /// Katman 1'in ortasındaki interaktif harita",
    '''    /// Katman 1'in üst şeridi: veri kaynağı ve WhatsApp günlük freni.
    pub(crate) fn kota_seridi(&mut self, ctx: &egui::Context) {
        let bugun = bugun_anahtari();
        let gonderilen = self.defter.gun_gonderim(&bugun);
        egui::TopBottomPanel::top("kota_seridi")
            .exact_height(46.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(22.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.label(egui::RichText::new("VERİ KAYNAĞI").size(9.0).color(DIM));
                    ui.label(
                        egui::RichText::new("OpenStreetMap · Overpass")
                            .size(10.5)
                            .color(ui.visuals().strong_text_color()),
                    );
                    ui.label(
                        egui::RichText::new("anahtarsız · ücretsiz · günlük kota yok")
                            .size(9.5)
                            .color(YESIL),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let kalan = GUNLUK_GONDERIM_LIMITI.saturating_sub(gonderilen);
                        let renk = if kalan == 0 {
                            KIRMIZI
                        } else if kalan <= 3 {
                            TURUNCU
                        } else {
                            YESIL
                        };
                        pill(
                            ui,
                            &format!("WhatsApp {gonderilen}/{GUNLUK_GONDERIM_LIMITI}"),
                            renk,
                        );
                    });
                });
            });
    }

''',
    ad="kota şeridi",
)
s = rep(
    s,
    '''            "{sektor} · {} kayıt · {} koordinatsız · Google Places",''',
    '''            "{sektor} · {} kayıt · {} koordinatsız · OpenStreetMap",''',
    ad="harita alt başlığı",
)
s = rep(
    s,
    "    /// Katman 1'in ortasındaki interaktif harita (Google Maps Static).",
    "    /// Katman 1'in ortasındaki interaktif harita (OSM döşemeleri).",
    ad="harita doc",
)
s = fn_sil(s, "batarya")
yaz("src/katmanlar.rs", s)

# --- saglik.rs -------------------------------------------------------------
s = yukle("src/saglik.rs")
s = kes(
    s,
    "/// Google Maps erişimi + Places/Static Maps anahtarının durumu.",
    "/// Dosya yoksa/boşsa sorun değil",
    '''/// OpenStreetMap erişimi: Overpass API ve döşeme sunucusu.
///
/// Anahtar, kota veya faturalı servis yoktur; tek gereksinim ağ erişimidir.
/// Ağ yoksa denetim kırmızıya döner ve tarama başlatılamaz.
fn harita() -> Result<String, String> {
    let erisim = |adres: &str| -> bool {
        let Ok(mut adresler) = adres.to_socket_addrs() else {
            return false;
        };
        let Some(soket) = adresler.next() else {
            return false;
        };
        TcpStream::connect_timeout(&soket, Duration::from_secs(3)).is_ok()
    };
    if !(erisim("overpass-api.de:443")
        || erisim("overpass.kumi.systems:443")
        || erisim("tile.openstreetmap.org:443"))
    {
        return Err(HARITA_HATA.to_string());
    }
    Ok("OpenStreetMap hazır: Overpass ve döşeme sunucusu erişilebilir (anahtarsız).".to_string())
}

''',
    ad="saglik harita",
)
yaz("src/saglik.rs", s)

# --- iller.rs --------------------------------------------------------------
s = yukle("src/iller.rs")
s = rep(
    s,
    "//! (volkansenturk/turkiye-iller-ilceler); harita sorgusu Google Geocoding ile yapılır.",
    "//! (volkansenturk/turkiye-iller-ilceler); konum çözümü Nominatim (OSM) ile yapılır.",
    ad="iller doc",
)
yaz("src/iller.rs", s)

# --- demo_site.rs ----------------------------------------------------------
s = yukle("src/demo_site.rs")
s = rep(
    s,
    '''    let gomulu = format!("https://maps.google.com/maps?q={enlem:.5},{boylam:.5}&z=17&output=embed");
    let ac = format!("https://www.google.com/maps/search/?api=1&query={enlem:.5},{boylam:.5}");''',
    '''    // Anahtarsız OSM gömme haritası ve bağlantısı.
    let d = 0.008;
    let gomulu = format!(
        "https://www.openstreetmap.org/export/embed.html?bbox={:.5}%2C{:.5}%2C{:.5}%2C{:.5}&layer=mapnik&marker={enlem:.5}%2C{boylam:.5}",
        boylam - d,
        enlem - d,
        boylam + d,
        enlem + d
    );
    let ac = format!(
        "https://www.openstreetmap.org/?mlat={enlem:.5}&mlon={boylam:.5}#map=17/{enlem:.5}/{boylam:.5}"
    );''',
    ad="demo harita",
)
s = s.replace('kalemler.push(("Google".to_string()', 'kalemler.push(("Yorumlar".to_string()')
s = rep(
    s,
    '    /// Galeri kaynağının kısa açıklaması (ör. "Google Street View · işletmenin',
    '    /// Galeri kaynağının kısa açıklaması (ör. "işletmenin fotoğrafları ·',
    ad="demo galeri doc",
)
yaz("src/demo_site.rs", s)

print("tamam: katmanlar, saglik, iller, demo_site")
