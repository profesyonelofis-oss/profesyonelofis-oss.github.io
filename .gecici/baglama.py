import io, sys, pathlib

kok = pathlib.Path(".")
hata = []


def oku(p):
    return (kok / p).read_text(encoding="utf-8")


def yaz(p, s):
    (kok / p).write_text(s, encoding="utf-8", newline="\n")


def degistir(p, eski, yeni, adet=1):
    s = oku(p)
    if s.count(eski) < 1:
        hata.append(f"{p}: aranan metin yok -> {eski[:60]!r}")
        return
    yaz(p, s.replace(eski, yeni, adet))


# --- 1. gelir.rs: arayüz bloğunu yeni tek-panel sürümüyle değiştir ----------
s = oku("src/gelir.rs")
ui = oku(".gecici/gelir_ui.rs")
uretim = s.find("impl LeadApp {")
testler = s.find("#[cfg(test)]")
if uretim < 0 or testler < 0:
    hata.append("gelir.rs: impl/test işaretleri bulunamadı")
else:
    bas = s[:uretim].replace("pakit_fiyat_yaz", "fiyat_yaz")
    son = s[testler:]
    yaz("src/gelir.rs", bas + ui.strip() + "\n\n" + son)

# --- 2. main.rs: modül, katman, alanlar, başlangıç değerleri ----------------
degistir("src/main.rs", "mod musteriler;\nmod nap;", "mod gelir;\nmod musteriler;\nmod nap;")

degistir(
    "src/main.rs",
    "enum Katman {\n    MusteriBulma,\n    WhatsApp,\n}\n\nimpl Katman {\n    const HEPSI: [Katman; 2] = [Katman::MusteriBulma, Katman::WhatsApp];",
    "enum Katman {\n    MusteriBulma,\n    WhatsApp,\n    Gelir,\n}\n\nimpl Katman {\n    const HEPSI: [Katman; 3] = [Katman::MusteriBulma, Katman::WhatsApp, Katman::Gelir];",
)

degistir(
    "src/main.rs",
    '            Katman::MusteriBulma => "Müşteri Bulma",\n            Katman::WhatsApp => "WhatsApp",',
    '            Katman::MusteriBulma => "Müşteri Bulma",\n            Katman::WhatsApp => "WhatsApp",\n            Katman::Gelir => "Gelir",',
)

degistir(
    "src/main.rs",
    "    denetim_panel_acik: bool,\n    denetim_liste: Vec<nap::Satir>,",
    "    denetim_panel_acik: bool,\n    denetim_liste: Vec<nap::Satir>,\n"
    "    /// Gelir katmanı: hizmet kataloğu, paketler ve abonelik kayıtları.\n"
    "    gelir: gelir::Gelir,\n"
    "    gelir_secili: Option<String>,\n"
    "    gelir_arama: String,\n"
    "    /// Seçili gelir hedefi (HEDEFLER dizisindeki sıra).\n"
    "    gelir_hedef: usize,",
)

degistir(
    "src/main.rs",
    "            denetim_panel_acik: false,\n            denetim_liste: Vec::new(),",
    "            denetim_panel_acik: false,\n            denetim_liste: Vec::new(),\n"
    "            gelir: gelir::Gelir::yukle(),\n"
    "            gelir_secili: None,\n"
    "            gelir_arama: String::new(),\n"
    "            gelir_hedef: 0,",
)

# --- 3. katmanlar.rs: Gelir sekmesinin ekranı -------------------------------
degistir(
    "src/katmanlar.rs",
    "            Katman::MusteriBulma => {}\n            Katman::WhatsApp => self.gelen_ekrani(ctx),",
    "            Katman::MusteriBulma => {}\n            Katman::WhatsApp => self.gelen_ekrani(ctx),\n            Katman::Gelir => self.gelir_ekrani(ctx),",
)

if hata:
    print("HATALAR:")
    for x in hata:
        print(" ", x)
    sys.exit(1)
print("baglama tamam")
