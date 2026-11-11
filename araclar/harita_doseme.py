"""Haritaya Google Static Maps döşemesi ekler: anahtar yalnız okunur, silinmez."""
import sys

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
p = "src/harita.rs"
s = open(p, encoding="utf-8").read()


def degistir(eski, yeni, ad):
    global s
    assert s.count(eski) == 1, f"{ad}: {s.count(eski)} eslesme"
    s = s.replace(eski, yeni, 1)


# 1) Importlar + sabitler.
degistir(
    "use std::time::Duration;",
    "use std::io::Read;\n"
    "use std::sync::mpsc::{channel, Receiver, TryRecvError};\n"
    "use std::time::{Duration, Instant};",
    "importlar",
)

degistir(
    "const VARSAYILAN_ZOOM: u8 = 16;",
    "const VARSAYILAN_ZOOM: u8 = 16;\n"
    "\n"
    "/// Google Static Maps dosemesi: 640x640 px, tek istekte gorunumu kaplar.\n"
    "const DOSEME_BOYUT: u32 = 640;\n"
    "/// Kamera dosemenin %30'undan fazla kaydiysa yeni doseme istenir.\n"
    "const DOSEME_KAYDIRMA: f32 = 0.30;\n"
    "/// Iki doseme istegi arasindaki en kisa sure.\n"
    "const DOSEME_ARALIK: Duration = Duration::from_secs(4);\n"
    "/// Oturum basina en fazla doseme istegi (kota korumasi).\n"
    "const DOSEME_OTURUM_SINIRI: u32 = 60;\n"
    "/// Ayni gorunumde ust uste hata sonrasi istek durur.\n"
    "const DOSEME_DENEME_SINIRI: u32 = 3;\n"
    "/// Harita hata rengi.\n"
    "const HATA: egui::Color32 = egui::Color32::from_rgb(217, 48, 37);",
    "sabitler",
)

# 2) Doseme durumu.
degistir(
    "pub struct Harita {\n    merkez: (f64, f64),\n    zoom: u8,\n    stil: Stil,\n}",
    "/// Yuklenmis Google Maps dosemesi ve hangi gorunume ait oldugu.\n"
    "struct Doku {\n"
    "    merkez: (f64, f64),\n"
    "    zoom: u8,\n"
    "    stil: Stil,\n"
    "    boyut: u32,\n"
    "    doku: egui::TextureHandle,\n"
    "}\n"
    "\n"
    "pub struct Harita {\n"
    "    merkez: (f64, f64),\n"
    "    zoom: u8,\n"
    "    stil: Stil,\n"
    "    /// Kasadan okunan API anahtari; silinmez, yalniz okunur.\n"
    "    anahtar: Option<String>,\n"
    "    /// Hazir doseme (varsa sematik cizimin uzerine basilir).\n"
    "    doku: Option<Doku>,\n"
    "    istek: Option<Receiver<Result<egui::ColorImage, String>>>,\n"
    "    istek_merkez: (f64, f64),\n"
    "    istek_zoom: u8,\n"
    "    istek_stil: Stil,\n"
    "    son_istek: Option<Instant>,\n"
    "    son_hareket: Option<Instant>,\n"
    "    deneme: u32,\n"
    "    doseme_sayaci: u32,\n"
    "    hata: Option<String>,\n"
    "}",
    "Harita alanlari",
)

degistir(
    "        Self {\n"
    "            merkez: EV,\n"
    "            zoom: VARSAYILAN_ZOOM,\n"
    "            stil: Stil::Vektor,\n"
    "        }",
    "        Self {\n"
    "            merkez: EV,\n"
    "            zoom: VARSAYILAN_ZOOM,\n"
    "            stil: Stil::Vektor,\n"
    "            anahtar: None,\n"
    "            doku: None,\n"
    "            istek: None,\n"
    "            istek_merkez: EV,\n"
    "            istek_zoom: VARSAYILAN_ZOOM,\n"
    "            istek_stil: Stil::Vektor,\n"
    "            son_istek: None,\n"
    "            son_hareket: None,\n"
    "            deneme: 0,\n"
    "            doseme_sayaci: 0,\n"
    "            hata: None,\n"
    "        }",
    "Harita varsayilanlari",
)

# 3) Doseme yonetimi.
degistir(
    "    pub fn stil(&self) -> Stil {\n        self.stil\n    }",
    "    pub fn stil(&self) -> Stil {\n"
    "        self.stil\n"
    "    }\n"
    "\n"
    "    /// Kasadaki anahtari bir kez okur. Anahtar silinmez; gorunum degisse de\n"
    "    /// (Vektor <-> Uydu, zoom, kaydirma) ayni anahtar kullanilmaya devam eder.\n"
    "    pub fn anahtar_yukle(&mut self) {\n"
    "        if self.anahtar.is_some() {\n"
    "            return;\n"
    "        }\n"
    "        self.anahtar = crate::anahtar::Kasa::yukle()\n"
    "            .anahtar()\n"
    "            .map(|anahtar| anahtar.to_string());\n"
    "    }\n"
    "\n"
    "    /// Google Static Maps adresi; anahtar yalniz istek URL'sinde kullanilir.\n"
    "    fn doseme_url(&self, anahtar: &str) -> String {\n"
    "        let tur = match self.stil {\n"
    "            Stil::Vektor => \"roadmap\",\n"
    "            Stil::Uydu => \"satellite\",\n"
    "        };\n"
    "        format!(\n"
    "            \"https://maps.googleapis.com/maps/api/staticmap?center={:.6},{:.6}&zoom={}&size={}x{}&scale=1&maptype={}&language=tr&region=TR&key={}\",\n"
    "            self.merkez.0, self.merkez.1, self.zoom, DOSEME_BOYUT, DOSEME_BOYUT, tur, anahtar\n"
    "        )\n"
    "    }\n"
    "\n"
    "    /// Eldeki doseme bu gorunumu hala karsiliyor mu?\n"
    "    fn doseme_gerekli(&self) -> bool {\n"
    "        let Some(doku) = &self.doku else {\n"
    "            return true;\n"
    "        };\n"
    "        if doku.zoom != self.zoom || doku.stil != self.stil {\n"
    "            return true;\n"
    "        }\n"
    "        let (ax, ay) = dunya_xy(doku.merkez.0, doku.merkez.1, self.zoom);\n"
    "        let (bx, by) = dunya_xy(self.merkez.0, self.merkez.1, self.zoom);\n"
    "        let fark = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();\n"
    "        fark > DOSEME_BOYUT as f64 * DOSEME_KAYDIRMA as f64\n"
    "    }\n"
    "\n"
    "    /// Dosemeyi arka planda indirir (arayuz kilitlenmez).\n"
    "    fn doseme_iste(&mut self) {\n"
    "        let Some(anahtar) = self.anahtar.clone() else {\n"
    "            return;\n"
    "        };\n"
    "        let url = self.doseme_url(&anahtar);\n"
    "        let (gonder, al) = channel();\n"
    "        std::thread::spawn(move || {\n"
    "            let _ = gonder.send(doseme_indir(&url));\n"
    "        });\n"
    "        self.istek = Some(al);\n"
    "        self.istek_merkez = self.merkez;\n"
    "        self.istek_zoom = self.zoom;\n"
    "        self.istek_stil = self.stil;\n"
    "        self.son_istek = Some(Instant::now());\n"
    "        self.doseme_sayaci += 1;\n"
    "    }\n"
    "\n"
    "    /// Tamamlanan istegi dokuya cevirir.\n"
    "    fn doseme_bekle(&mut self, ctx: &egui::Context) {\n"
    "        let Some(al) = self.istek.take() else {\n"
    "            return;\n"
    "        };\n"
    "        match al.try_recv() {\n"
    "            Ok(Ok(resim)) => {\n"
    "                let doku = ctx.load_texture(\"google_harita\", resim, egui::TextureOptions::LINEAR);\n"
    "                self.doku = Some(Doku {\n"
    "                    merkez: self.istek_merkez,\n"
    "                    zoom: self.istek_zoom,\n"
    "                    stil: self.istek_stil,\n"
    "                    boyut: DOSEME_BOYUT,\n"
    "                    doku,\n"
    "                });\n"
    "                self.hata = None;\n"
    "                self.deneme = 0;\n"
    "            }\n"
    "            Ok(Err(hata)) => {\n"
    "                self.hata = Some(hata);\n"
    "                self.deneme += 1;\n"
    "            }\n"
    "            Err(TryRecvError::Empty) => {\n"
    "                self.istek = Some(al);\n"
    "            }\n"
    "            Err(TryRecvError::Disconnected) => {\n"
    "                self.hata = Some(\"doseme istegi kesildi\".to_string());\n"
    "                self.deneme += 1;\n"
    "            }\n"
    "        }\n"
    "    }\n"
    "\n"
    "    /// Gerekliyse (kamera durdugunda) yeni doseme ister.\n"
    "    fn doseme_tazele(&mut self, hareket: bool) {\n"
    "        if hareket {\n"
    "            self.son_hareket = Some(Instant::now());\n"
    "            return;\n"
    "        }\n"
    "        if self.istek.is_some() || self.anahtar.is_none() {\n"
    "            return;\n"
    "        }\n"
    "        if self.deneme >= DOSEME_DENEME_SINIRI || self.doseme_sayaci >= DOSEME_OTURUM_SINIRI {\n"
    "            return;\n"
    "        }\n"
    "        if self\n"
    "            .son_istek\n"
    "            .is_some_and(|zaman| zaman.elapsed() < DOSEME_ARALIK)\n"
    "        {\n"
    "            return;\n"
    "        }\n"
    "        if self\n"
    "            .son_hareket\n"
    "            .is_some_and(|zaman| zaman.elapsed() < Duration::from_millis(450))\n"
    "        {\n"
    "            return;\n"
    "        }\n"
    "        if self.doseme_gerekli() {\n"
    "            self.doseme_iste();\n"
    "        }\n"
    "    }\n"
    "\n"
    "    /// Dosemenin ekrana oturacagi dikdortgen; kamera kayinca kendiliginden\n"
    "    /// kayar, boylece her kaydirmada yeni istek gerekmez.\n"
    "    fn resim_yerlesimi(&self, rect: egui::Rect) -> Option<(egui::TextureId, egui::Rect)> {\n"
    "        let doku = self.doku.as_ref()?;\n"
    "        if doku.zoom != self.zoom || doku.stil != self.stil {\n"
    "            return None;\n"
    "        }\n"
    "        let (ix, iy) = dunya_xy(doku.merkez.0, doku.merkez.1, self.zoom);\n"
    "        let (cx, cy) = dunya_xy(self.merkez.0, self.merkez.1, self.zoom);\n"
    "        let merkez = egui::pos2(\n"
    "            rect.center().x + (ix - cx) as f32,\n"
    "            rect.center().y + (iy - cy) as f32,\n"
    "        );\n"
    "        let boyut = doku.boyut as f32;\n"
    "        Some((\n"
    "            doku.doku.id(),\n"
    "            egui::Rect::from_center_size(merkez, egui::vec2(boyut, boyut)),\n"
    "        ))\n"
    "    }",
    "doseme yonetimi",
)

# 4) ciz: durum okuma + indirme tetigi.
degistir(
    "        if rect.width() < 60.0 || rect.height() < 60.0 {\n            return None;\n        }",
    "        if rect.width() < 60.0 || rect.height() < 60.0 {\n"
    "            return None;\n"
    "        }\n"
    "        self.anahtar_yukle();\n"
    "        self.doseme_bekle(ui.ctx());",
    "doseme durumu",
)

degistir(
    "        // --- Zemin, adalar ve yollar --------------------------------------",
    "        // Kamera durduysa ve gorunum degistiyse doseme istenir.\n"
    "        self.doseme_tazele(yanit.dragged());\n"
    "\n"
    "        // --- Zemin, adalar ve yollar --------------------------------------",
    "doseme tetigi",
)

# 5) Dosemeyi sematik cizimin uzerine bas.
degistir(
    "        // --- Başlık şeridi ------------------------------------------------",
    "        // --- Google Maps dosemesi (varsa) ---------------------------------\n"
    "        if let Some((doku, kutu)) = self.resim_yerlesimi(rect) {\n"
    "            p.image(\n"
    "                doku,\n"
    "                kutu,\n"
    "                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),\n"
    "                egui::Color32::WHITE,\n"
    "            );\n"
    "        }\n"
    "\n"
    "        // --- Başlık şeridi ------------------------------------------------",
    "doseme cizimi",
)

# 6) Durum notu.
degistir(
    "        p.text(\n"
    "            rect.right_bottom() - egui::vec2(14.0, 12.0),\n"
    "            egui::Align2::RIGHT_BOTTOM,\n"
    "            \"Google Places verisi · yerel şematik çizim · ek istek yok\",\n"
    "            egui::FontId::proportional(8.5),\n"
    "            stil.soluk(),\n"
    "        );",
    "        let (durum, durum_rengi) = if let Some(hata) = &self.hata {\n"
    "            (format!(\"Harita döşemesi alınamadı: {hata}\"), HATA)\n"
    "        } else if self.istek.is_some() {\n"
    "            (\n"
    "                \"Google Maps döşemesi yükleniyor…\".to_string(),\n"
    "                stil.soluk(),\n"
    "            )\n"
    "        } else if self.doku.is_some() {\n"
    "            (\n"
    "                format!(\n"
    "                    \"© Google Maps · döşeme {} / {} · şematik yedek hazır\",\n"
    "                    self.doseme_sayaci, DOSEME_OTURUM_SINIRI\n"
    "                ),\n"
    "                stil.soluk(),\n"
    "            )\n"
    "        } else if self.anahtar.is_none() {\n"
    "            (\n"
    "                \"Harita anahtarı yok · API & Kota panelinden girin\".to_string(),\n"
    "                HATA,\n"
    "            )\n"
    "        } else {\n"
    "            (\n"
    "                \"Google Maps döşemesi bekleniyor · şematik görünüm\".to_string(),\n"
    "                stil.soluk(),\n"
    "            )\n"
    "        };\n"
    "        p.text(\n"
    "            rect.right_bottom() - egui::vec2(14.0, 12.0),\n"
    "            egui::Align2::RIGHT_BOTTOM,\n"
    "            durum,\n"
    "            egui::FontId::proportional(8.5),\n"
    "            durum_rengi,\n"
    "        );",
    "durum notu",
)

degistir(
    "        if yanit.hovered() {\n            ui.ctx().request_repaint_after(Duration::from_millis(120));\n        }",
    "        if yanit.hovered() || self.istek.is_some() {\n"
    "            ui.ctx().request_repaint_after(Duration::from_millis(150));\n"
    "        }",
    "yeniden cizim",
)

# 7) Indirme yardimcisi.
degistir(
    "#[cfg(test)]\nmod testler {",
    "/// Static Maps yanitini gosterilebilir goruntuye cevirir.\n"
    "///\n"
    "/// Anahtar URL'de kalir; hata metnine anahtar yazilmaz.\n"
    "fn doseme_indir(url: &str) -> Result<egui::ColorImage, String> {\n"
    "    let yanit = match ureq::get(url).timeout(Duration::from_secs(10)).call() {\n"
    "        Ok(yanit) => yanit,\n"
    "        Err(ureq::Error::Status(_, yanit)) => yanit,\n"
    "        Err(hata) => return Err(format!(\"ağ hatası: {hata}\")),\n"
    "    };\n"
    "    let icerik = yanit\n"
    "        .header(\"content-type\")\n"
    "        .unwrap_or_default()\n"
    "        .to_ascii_lowercase();\n"
    "    let mut bayt = Vec::new();\n"
    "    yanit\n"
    "        .into_reader()\n"
    "        .take(4_000_000)\n"
    "        .read_to_end(&mut bayt)\n"
    "        .map_err(|hata| format!(\"yanıt okunamadı: {hata}\"))?;\n"
    "    if !icerik.contains(\"image\") {\n"
    "        let metin = String::from_utf8_lossy(&bayt);\n"
    "        let mesaj = serde_json::from_str::<serde_json::Value>(&metin)\n"
    "            .ok()\n"
    "            .and_then(|json| {\n"
    "                json\n"
    "                    .get(\"error_message\")\n"
    "                    .or_else(|| json.get(\"status\"))\n"
    "                    .and_then(|deger| deger.as_str())\n"
    "                    .map(|metin| metin.to_string())\n"
    "            })\n"
    "            .unwrap_or_else(|| \"görüntü yerine metin yanıt geldi\".to_string());\n"
    "        return Err(mesaj);\n"
    "    }\n"
    "    let resim =\n"
    "        image::load_from_memory(&bayt).map_err(|hata| format!(\"görüntü çözülemedi: {hata}\"))?;\n"
    "    let rgba = resim.to_rgba8();\n"
    "    let boyut = [rgba.width() as usize, rgba.height() as usize];\n"
    "    Ok(egui::ColorImage::from_rgba_unmultiplied(\n"
    "        boyut,\n"
    "        rgba.as_raw(),\n"
    "    ))\n"
    "}\n"
    "\n"
    "#[cfg(test)]\n"
    "mod testler {",
    "indirme yardimcisi",
)

degistir(
    "    fn stil_degisir_ve_etiket_doner() {",
    "    #[test]\n"
    "    #[ignore = \"canlı Google Static Maps isteği: gerçek ağ ve kota harcar\"]\n"
    "    fn canli_doseme_erisimi() {\n"
    "        let kasa = crate::anahtar::Kasa::yukle();\n"
    "        let anahtar = kasa.anahtar().expect(\"kasa boş: panelden anahtar girin\");\n"
    "        let harita = Harita::default();\n"
    "        let url = harita.doseme_url(anahtar);\n"
    "        match doseme_indir(&url) {\n"
    "            Ok(resim) => println!(\"statik harita tamam: {}x{} px\", resim.width(), resim.height()),\n"
    "            Err(hata) => println!(\"statik harita hatası: {hata}\"),\n"
    "        }\n"
    "    }\n"
    "\n"
    "    #[test]\n"
    "    fn stil_degisir_ve_etiket_doner() {",
    "canli test",
)

open(p, "w", encoding="utf-8", newline="\n").write(s)
print("harita: Google dosemesi eklendi")
