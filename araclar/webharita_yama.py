"""Gömülü Google Maps JavaScript API katmanı (wry/WebView2) yaması."""
import io
import sys

sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def oku(yol):
    return io.open(yol, encoding="utf-8").read()


def yaz(yol, metin):
    io.open(yol, "w", encoding="utf-8", newline="\n").write(metin)


def degistir(metin, eski, yeni, ad):
    assert metin.count(eski) == 1, f"{ad}: {metin.count(eski)} eslesme"
    return metin.replace(eski, yeni, 1)


# ===========================================================================
# src/webharita.rs
# ===========================================================================
MODUL = r'''//! Sağ panelde gömülü Google Maps JavaScript API katmanı.
//!
//! egui'nin kendi çizimi yerine WebView2 (wry) alt penceresi kullanılır:
//! harita, Google'ın vektör döşemeleri ve JS API denetimleriyle tam
//! etkileşimlidir (pan, zoom, pin, bilgi penceresi). WebView2 çalışma zamanı
//! yoksa veya anahtar geçersizse `olustur` hata döner ve çağıran taraf yerel
//! şematik çizime düşer; uygulama hiçbir durumda durmaz.

use eframe::egui;
use std::num::NonZeroIsize;
use wry::raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};

/// Manisa · Şehzadeler: Şelale Park / Atatürk Kent Parkı / MASKİ çevresi.
pub const MERKEZ: (f64, f64) = (38.6128, 27.4265);
pub const ZOOM: u8 = 14;

/// eframe penceresinin HWND'sini wry'ye taşıyan ince sarmalayıcı.
struct Pencere(isize);

impl HasWindowHandle for Pencere {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let Some(hwnd) = NonZeroIsize::new(self.0) else {
            return Err(HandleError::Unavailable);
        };
        // SAFETY: HWND eframe penceresine aittir ve uygulama yaşadıkça geçerlidir.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(hwnd))) })
    }
}

fn fizik(konum: egui::Rect) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::PhysicalPosition::new(konum.min.x as i32, konum.min.y as i32).into(),
        size: wry::dpi::PhysicalSize::new(
            konum.width().max(1.0) as u32,
            konum.height().max(1.0) as u32,
        )
        .into(),
    }
}

/// Gömülü harita: WebView2 alt penceresi ve son sınırları.
pub struct WebHarita {
    webview: wry::WebView,
    sinir: egui::Rect,
    gorunur: bool,
}

impl WebHarita {
    /// Alt pencereyi oluşturur; HTML, Maps JavaScript API'yi anahtarla yükler.
    pub fn olustur(hwnd: isize, konum: egui::Rect, anahtar: &str) -> Result<Self, String> {
        let guvenli: String = anahtar
            .chars()
            .filter(|harf| harf.is_ascii_alphanumeric() || *harf == '-' || *harf == '_')
            .collect();
        if guvenli.len() < 20 {
            return Err("API anahtarı geçersiz görünüyor.".to_string());
        }
        let webview = wry::WebViewBuilder::new()
            .with_html(harita_html(&guvenli))
            .with_bounds(fizik(konum))
            .with_background_color((242, 243, 245, 255))
            .with_devtools(false)
            .build_as_child(&Pencere(hwnd))
            .map_err(|hata| format!("WebView2 açılamadı: {hata}"))?;
        Ok(Self {
            webview,
            sinir: konum,
            gorunur: true,
        })
    }

    /// Panel dikdörtgeni değiştiyse alt pencereyi aynı sınıra taşır.
    pub fn sinir_ayarla(&mut self, konum: egui::Rect) -> Result<(), String> {
        let ayni = (self.sinir.min - konum.min).length() < 1.0
            && (self.sinir.size() - konum.size()).length() < 1.0;
        if ayni {
            return Ok(());
        }
        self.webview
            .set_bounds(fizik(konum))
            .map_err(|hata| format!("Harita sınırı uygulanamadı: {hata}"))?;
        self.sinir = konum;
        Ok(())
    }

    /// Haritayı gösterir/gizler (katman değişimi ve açık pencereler için).
    pub fn gorunurluk(&mut self, gorunur: bool) -> Result<(), String> {
        if self.gorunur == gorunur {
            return Ok(());
        }
        self.webview
            .set_visible(görünür_metin(gorunur))
            .map_err(|hata| format!("Harita görünürlüğü değiştirilemedi: {hata}"))?;
        self.gorunur = gorunur;
        Ok(())
    }

    /// Web sitesi olmayan işletmeleri haritaya pin olarak işler.
    pub fn pinler(&self, leads: &[crate::BusinessLead]) -> Result<(), String> {
        let mut liste = Vec::new();
        for lead in leads {
            let Some((enlem, boylam)) = lead.konum else {
                continue;
            };
            if !(-90.0..=90.0).contains(&enlem) || !(-180.0..=180.0).contains(&boylam) {
                continue;
            }
            let alt = match (lead.phone.trim(), lead.address.trim()) {
                ("", "") => String::new(),
                ("", adres) => adres.to_string(),
                (telefon, "") => telefon.to_string(),
                (telefon, adres) => format!("{telefon} · {adres}"),
            };
            liste.push(serde_json::json!({
                "sira": liste.len(),
                "ad": lead.name,
                "etiket": lead.category,
                "alt": alt,
                "lat": enlem,
                "lng": boylam,
            }));
        }
        let govde = serde_json::to_string(&liste)
            .map_err(|hata| format!("Pin verisi hazırlanamadı: {hata}"))?;
        self.webview
            .evaluate_script(&format!(
                "window.nexusPinler && window.nexusPinler({govde});"
            ))
            .map_err(|hata| format!("Pinler işlenemedi: {hata}"))
    }

    /// Listeden seçilen işletmenin pinine odaklanıp bilgi penceresini açar.
    pub fn sec(&self, sira: usize) -> Result<(), String> {
        self.webview
            .evaluate_script(&format!("window.nexusSec && window.nexusSec({sira});"))
            .map_err(|hata| format!("Harita seçimi uygulanamadı: {hata}"))
    }

    /// Haritayı yeni sonuç kapsamına yumuşakça taşır.
    pub fn merkez(&self, enlem: f64, boylam: f64, zoom: u8) -> Result<(), String> {
        self.webview
            .evaluate_script(&format!(
                "window.nexusMerkez && window.nexusMerkez({enlem:.6}, {boylam:.6}, {zoom});"
            ))
            .map_err(|hata| format!("Harita merkezi uygulanamadı: {hata}"))
    }
}

fn görünür_metin(gorunur: bool) -> bool {
    gorunur
}

/// Gömülü sayfa: Google Maps JavaScript API + pinler + bilgi penceresi.
fn harita_html(anahtar: &str) -> String {
    let sablon = r##"<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
  html,body,#m{height:100%;width:100%;margin:0;padding:0;background:#f2f3f5;overflow:hidden}
  #hata{display:none;position:absolute;inset:0;padding:18px;font:13px/1.5 Roboto,Arial,sans-serif;color:#5f6368;background:#fff}
  #hata b{color:#c5221f}
</style>
</head>
<body>
<div id="m"></div>
<div id="hata">
  <b>Google Maps yüklenemedi.</b><br>
  Google Cloud Console'da <b>Maps JavaScript API</b> etkin mi ve faturalandırma açık mı?
  Anahtar kısıtı "HTTP referrer" ise boş bırakılmalı (gömülü görünümde origin yoktur).
</div>
<script>
function nexusBasla() {
  window.nexusHazir = true;
  var harita = new google.maps.Map(document.getElementById('m'), {
    center: {lat: 38.6128, lng: 27.4265},
    zoom: 14,
    mapTypeId: 'roadmap',
    mapTypeControl: true,
    streetViewControl: false,
    fullscreenControl: true,
    clickableIcons: false,
    gestureHandling: 'greedy'
  });
  var pinler = [];
  var secili = null;

  function kart(veri) {
    var kutu = document.createElement('div');
    kutu.style.cssText = 'font:12px/1.45 Roboto,Arial,sans-serif;color:#202124;max-width:230px';
    var ad = document.createElement('div');
    ad.textContent = veri.ad || '';
    ad.style.cssText = 'font-weight:600;font-size:13px';
    kutu.appendChild(ad);
    if (veri.etiket) {
      var tur = document.createElement('div');
      tur.textContent = veri.etiket;
      tur.style.color = '#5f6368';
      kutu.appendChild(tur);
    }
    if (veri.alt) {
      var alt = document.createElement('div');
      alt.textContent = veri.alt;
      alt.style.cssText = 'color:#3c4043;margin-top:4px';
      kutu.appendChild(alt);
    }
    var rozet = document.createElement('div');
    rozet.textContent = 'Web sitesi yok · hedef müşteri';
    rozet.style.cssText = 'margin-top:6px;color:#b06000;font-weight:600';
    kutu.appendChild(rozet);
    return kutu;
  }

  function bilgiAc(pin) {
    if (secili) { secili.close(); }
    secili = pin.__bilgi;
    secili.open({map: harita, anchor: pin});
  }

  window.nexusPinler = function(liste) {
    pinler.forEach(function(pin){ pin.setMap(null); });
    pinler = [];
    secili = null;
    (liste || []).forEach(function(veri) {
      var pin = new google.maps.Marker({
        position: {lat: veri.lat, lng: veri.lng},
        map: harita,
        title: veri.ad || '',
        label: {text: String((veri.sira || 0) + 1), color: '#ffffff', fontSize: '10px', fontWeight: '700'}
      });
      pin.__bilgi = new google.maps.InfoWindow({content: kart(veri)});
      pin.addListener('click', function(){ bilgiAc(pin); });
      pinler.push(pin);
    });
  };

  window.nexusSec = function(sira) {
    var pin = pinler[sira];
    if (!pin) { return; }
    harita.panTo(pin.getPosition());
    if (harita.getZoom() < 16) { harita.setZoom(16); }
    bilgiAc(pin);
  };

  window.nexusMerkez = function(enlem, boylam, zoom) {
    harita.panTo({lat: enlem, lng: boylam});
    if (zoom) { harita.setZoom(zoom); }
  };
}
setTimeout(function(){
  if (!window.nexusHazir) { document.getElementById('hata').style.display = 'block'; }
}, 7000);
</script>
<script async defer
  src="https://maps.googleapis.com/maps/api/js?key=__ANAHTAR__&language=tr&region=TR&loading=async&callback=nexusBasla"></script>
</body>
</html>"##;
    sablon.replace("__ANAHTAR__", anahtar)
}
'''
yaz("src/webharita.rs", MODUL)
print("webharita.rs yazıldı")

# ===========================================================================
# main.rs
# ===========================================================================
m = oku("src/main.rs")

# Modül bildirimi.
assert "mod harita;" in m
m = degistir(m, "mod harita;", "mod harita;\nmod webharita;", "modül bildirimi")

# Alanlar.
m = degistir(
    m,
    "    /// Listeden seçilen işletme: harita bu pine yumuşak geçiş yapar.\n    harita_odak: Option<usize>,",
    "    /// Listeden seçilen işletme: harita bu pine yumuşak geçiş yapar.\n"
    "    harita_odak: Option<usize>,\n"
    "    /// Sağ paneldeki gömülü Google Maps JavaScript API (WebView2) katmanı.\n"
    "    harita_web: Option<webharita::WebHarita>,\n"
    "    /// WebView katmanı kurulamazsa nedeni; panelde gösterilir.\n"
    "    harita_web_hata: Option<String>,\n"
    "    /// Haritaya işlenen son pin imzası.\n"
    "    harita_web_imza: Option<(usize, Option<String>)>,\n"
    "    /// eframe ana penceresinin HWND'si (WebView2 alt penceresi için).\n"
    "    pencere_kolu: Option<isize>,",
    "harita_web alanları",
)
m = degistir(
    m,
    "            harita_odak: None,",
    "            harita_odak: None,\n"
    "            harita_web: None,\n"
    "            harita_web_hata: None,\n"
    "            harita_web_imza: None,\n"
    "            pencere_kolu: None,",
    "harita_web başlangıcı",
)

# run_native: HWND yakala.
m = degistir(
    m,
    "        Box::new(|cc| {\n            yazi_tipi_yukle(&cc.egui_ctx);\n            tema_uygula(&cc.egui_ctx);\n            Box::<LeadApp>::default()\n        }),",
    """        Box::new(|cc| {
            yazi_tipi_yukle(&cc.egui_ctx);
            tema_uygula(&cc.egui_ctx);
            let mut uygulama = LeadApp::default();
            // WebView2 alt penceresi ana pencerenin HWND'sine bağlanır.
            if let Ok(kol) = cc.window_handle() {
                if let raw_window_handle::RawWindowHandle::Win32(win32) = kol.as_raw() {
                    uygulama.pencere_kolu = Some(win32.hwnd.get());
                }
            }
            Box::new(uygulama)
        }),""",
    "run_native HWND",
)

# Katman değişince WebView gizlenir.
m = degistir(
    m,
    "        if self.katman != Katman::Kesif {\n            let gonderen = whatsapp_numarasi(&self.whatsapp_phone)",
    "        if self.katman != Katman::Kesif {\n            self.harita_web_gizle();\n            let gonderen = whatsapp_numarasi(&self.whatsapp_phone)",
    "katman değişimi gizleme",
)

# Harita paneli: WebView varsa Google JS API, yoksa yerel çizim.
m = degistir(
    m,
    "                ui.add_space(4.0);\n                let yukseklik = ui.available_height().max(200.0);\n                harita_tiklama = self.kesif_haritasi(ui, yukseklik);",
    """                ui.add_space(4.0);
                let alan = ui.available_rect_before_wrap();
                if self.harita_web_ciz(ui, alan) {
                    ui.allocate_rect(alan, egui::Sense::hover());
                } else {
                    let yukseklik = alan.height().max(200.0);
                    harita_tiklama = self.kesif_haritasi(ui, yukseklik);
                }""",
    "harita paneli web dalı",
)

# Web katmanı yönetimi (impl LeadApp içine).
assert m.count("impl eframe::App for LeadApp {") == 1
YONTEM = r'''    /// Sağ paneli gömülü Google Maps JavaScript API ile çizer.
    ///
    /// Başarılıysa `true` döner ve yerel şematik çizim atlanır. WebView2
    /// yoksa, anahtar boşsa ya da sayfa kurulamazsa `false` döner; panel
    /// mevcut yerel haritaya düşer ve neden üst şeritte gösterilir.
    fn harita_web_ciz(&mut self, ui: &mut egui::Ui, alan: egui::Rect) -> bool {
        if alan.width() < 120.0 || alan.height() < 120.0 {
            return false;
        }
        let olcek = ui.ctx().pixels_per_point();
        let fizik = egui::Rect::from_min_size(
            egui::pos2((alan.min.x * olcek).round(), (alan.min.y * olcek).round()),
            egui::vec2(
                (alan.width() * olcek).round().max(1.0),
                (alan.height() * olcek).round().max(1.0),
            ),
        );

        if self.harita_web.is_none() {
            let Some(kol) = self.pencere_kolu else {
                return false;
            };
            let Some(anahtar) = anahtar::Kasa::yukle().anahtar().map(str::to_string) else {
                return false;
            };
            match webharita::WebHarita::olustur(kol, fizik, &anahtar) {
                Ok(web) => {
                    self.harita_web = Some(web);
                    self.harita_web_hata = None;
                    self.harita_web_imza = None;
                }
                Err(hata) => {
                    self.harita_web_hata = Some(hata);
                    return false;
                }
            }
        }

        // Açık pencereler haritanın üstünde kalsın: WebView gizlenir.
        let pencere_acik = self.api_panel
            || self.sektor_panel_acik
            || self.takip_panel_acik
            || self.bugun_panel_acik
            || self.tema_panel_acik
            || self.whatsapp_acik;

        let Some(web) = self.harita_web.as_mut() else {
            return false;
        };
        if let Err(hata) = web.sinir_ayarla(fizik) {
            self.harita_web_hata = Some(hata);
        }
        if let Err(hata) = web.gorunurluk(!pencere_acik) {
            self.harita_web_hata = Some(hata);
        }

        // Pin listesi yalnız sonuç kümesi değiştiğinde yeniden işlenir.
        let imza = (self.leads.len(), self.leads.first().map(|lead| lead.name.clone()));
        if self.harita_web_imza.as_ref() != Some(&imza) {
            self.harita_web_imza = Some(imza);
            if let Err(hata) = web.pinler(&self.leads) {
                self.harita_web_hata = Some(hata);
            }
        }

        // Listeden seçilen işletme: haritada odak + bilgi penceresi.
        if let Some(idx) = self.harita_odak.take() {
            if let Some(lead) = self.leads.get(idx) {
                let sira = self
                    .leads
                    .iter()
                    .filter(|kayit| kayit.konum.is_some())
                    .take_while(|kayit| kayit.name != lead.name)
                    .count();
                if let Err(hata) = web.sec(sira) {
                    self.harita_web_hata = Some(hata);
                }
            }
        }
        true
    }

    /// WebView katmanını gizler (keşif dışı katmanlar).
    fn harita_web_gizle(&mut self) {
        if let Some(web) = self.harita_web.as_mut() {
            let _ = web.gorunurluk(false);
        }
    }

    /// API anahtarı değiştiğinde WebView katmanını yeniden kurar.
    pub(crate) fn harita_web_yenile(&mut self) {
        self.harita_web = None;
        self.harita_web_hata = None;
        self.harita_web_imza = None;
    }

'''
m = degistir(m, "impl eframe::App for LeadApp {", YONTEM + "impl eframe::App for LeadApp {", "harita_web yöntemleri")
yaz("src/main.rs", m)
print("main.rs: web harita entegrasyonu")

# ===========================================================================
# katmanlar.rs: anahtar değişiminde WebView yenile
# ===========================================================================
k = oku("src/katmanlar.rs")
k = k.replace(
    "                                self.harita.anahtar_yenile();",
    "                                self.harita.anahtar_yenile();\n                                self.harita_web_yenile();",
)
yaz("src/katmanlar.rs", k)
print("katmanlar.rs: web harita yenileme")
