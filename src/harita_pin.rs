//! Harita görünümü: tüm işletme pinlerini OpenStreetMap karoları üzerinde
//! gösterir, haritaya tıklanan noktayı yeni konum olarak döndürür.
//!
//! Karolar arka planda **paralel** indirilir; arayüz kilitlenmez. Sunucuya
//! ulaşılamazsa görünüm şematik konum çizimine düşer (mesafe halkaları,
//! ölçek, kuzey oku) ve pinler yine görünür. Her görünüm için tek otomatik
//! deneme yapılır; "Haritayı yenile" ile yeniden denenir.
//!
//! Harita yalnızca görüntüleme, seçim ve konum düzeltme içindir; işletme/pin
//! oluşturmaz.

use eframe::egui;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

/// Tek karo kenarı (OSM standardı).
const KARO: u32 = 256;
/// Görüntünün yarı genişliği: 512×512 px'lik alan istenir.
const YARI: f64 = 256.0;
/// OSM karo sunucusunun istediği tanıtıcı.
const TANITICI: &str = "NexusEnterprise/1.0 (masaustu pin araci)";
pub const EN_AZ_ZOOM: u8 = 9;
pub const EN_COK_ZOOM: u8 = 18;
/// Tıklamada pini seçmiş sayılma yarıçapı (piksel).
const VURMA: f32 = 13.0;

/// Haritada gösterilecek işletme işareti.
#[derive(Clone)]
pub struct Isaret {
    pub anahtar: String,
    pub konum: (f64, f64),
    pub secili: bool,
    /// Pinin yanında görünen kısa etiket (ör. sıra numarası).
    pub etiket: Option<String>,
}

/// Haritadaki tıklamanın sonucu.
pub enum Tiklama {
    /// Var olan bir pinin üstüne tıklandı.
    Isaret(String),
    /// Boş alana tıklandı: yeni konum taslağı.
    Konum((f64, f64)),
}

/// Tek bir görünüm için indirilen karo birleşimi.
struct Yuklenen {
    doku: egui::TextureHandle,
    /// Görüntünün sol üst köşesinin dünya pikseli (zoom'a göre).
    sol_ust: (f64, f64),
    boyut: egui::Vec2,
    zoom: u8,
}

/// Arka plandan gelen ham görüntü.
struct Ham {
    resim: egui::ColorImage,
    sol_ust: (f64, f64),
    zoom: u8,
}

pub struct Harita {
    merkez: (f64, f64),
    zoom: u8,
    yuklenen: Option<Yuklenen>,
    istek: Option<Receiver<Result<Ham, String>>>,
    hata: Option<String>,
    /// Bu görünüm için yapılan otomatik deneme sayısı.
    deneme: u32,
    /// İsteğin başladığı an: takılan istekler bu süre sonunda bırakılır.
    baslangic: Option<std::time::Instant>,
}

impl Default for Harita {
    fn default() -> Self {
        Self {
            merkez: (39.0, 35.0),
            zoom: 16,
            yuklenen: None,
            istek: None,
            hata: None,
            deneme: 0,
            baslangic: None,
        }
    }
}

impl Harita {
    /// Haritayı bir noktaya getirir; konum değişmediyse hiçbir şey yapmaz.
    pub fn yonlendir(&mut self, enlem: f64, boylam: f64) {
        if (self.merkez.0 - enlem).abs() < 1e-9 && (self.merkez.1 - boylam).abs() < 1e-9 {
            return;
        }
        self.merkez = (enlem, boylam);
        self.sifirla();
    }

    /// Tüm noktaları ekrana sığdıracak merkez ve zoom'u seçer.
    pub fn sigdir(&mut self, noktalar: &[(f64, f64)]) {
        self.sigdir_alana(noktalar, 380.0, 220.0);
    }

    /// Verilen piksel alanına sığdıracak merkez ve zoom'u seçer.
    pub fn sigdir_alana(&mut self, noktalar: &[(f64, f64)], genislik: f32, yukseklik: f32) {
        let Some(ilk) = noktalar.first().copied() else {
            return;
        };
        if noktalar.len() == 1 {
            self.merkez = ilk;
            self.zoom = 16;
            self.sifirla();
            return;
        }
        let (mut min_e, mut max_e, mut min_b, mut max_b) = (ilk.0, ilk.0, ilk.1, ilk.1);
        for (e, b) in noktalar {
            min_e = min_e.min(*e);
            max_e = max_e.max(*e);
            min_b = min_b.min(*b);
            max_b = max_b.max(*b);
        }
        let merkez = ((min_e + max_e) / 2.0, (min_b + max_b) / 2.0);
        // En yakın (en büyük) zoom'dan başla: sığan ilk değer seçilir.
        // Hiçbir zoom sığmazsa en geniş görünüm (6) kullanılır.
        let mut secilen = 6u8;
        // Uzun koridorlar (İzmir–İstanbul) 9. zoom'a sığmaz: 6'ya kadar inilir.
        for z in (6..=EN_COK_ZOOM).rev() {
            let (x0, y0) = dunya(min_e, min_b, z);
            let (x1, y1) = dunya(max_e, max_b, z);
            // Y ekseni güneyden kuzeye azaldığı için fark mutlak alınır.
            if (x1 - x0).abs() as f32 <= genislik && (y1 - y0).abs() as f32 <= yukseklik {
                secilen = z;
                break;
            }
        }
        self.merkez = merkez;
        self.zoom = secilen;
        self.sifirla();
    }

    /// Görünümü doğrudan ayarlar (seçilen pine odaklanma).
    pub fn ayarla(&mut self, enlem: f64, boylam: f64, zoom: u8) {
        self.merkez = (enlem, boylam);
        self.zoom = zoom.clamp(6, EN_COK_ZOOM);
        self.sifirla();
    }

    pub fn yakinlas(&mut self) {
        if self.zoom < EN_COK_ZOOM {
            self.zoom += 1;
            self.sifirla();
        }
    }

    pub fn uzaklas(&mut self) {
        if self.zoom > EN_AZ_ZOOM {
            self.zoom -= 1;
            self.sifirla();
        }
    }

    /// Kullanıcı isteğiyle yeniden indirme.
    pub fn yenile(&mut self) {
        self.istek = None;
        self.hata = None;
        self.deneme = 0;
        self.baslangic = None;
    }

    pub fn zoom(&self) -> u8 {
        self.zoom
    }

    fn sifirla(&mut self) {
        self.yuklenen = None;
        self.istek = None;
        self.hata = None;
        self.deneme = 0;
        self.baslangic = None;
    }

    /// Haritayı çizer. `isaretler` haritadaki tüm işletmeler, `taslak` ise
    /// henüz kaydedilmemiş yeni konumdur.
    pub fn goster(
        &mut self,
        ui: &mut egui::Ui,
        boyut: egui::Vec2,
        isaretler: &[Isaret],
        taslak: Option<(f64, f64)>,
    ) -> Option<Tiklama> {
        self.goster_cizgi(ui, boyut, isaretler, &[], taslak)
    }

    /// İşaretler arasına hat çizerek gösterir (otoyol koridoru gibi).
    pub fn goster_cizgi(
        &mut self,
        ui: &mut egui::Ui,
        boyut: egui::Vec2,
        isaretler: &[Isaret],
        cizgi: &[(f64, f64)],
        taslak: Option<(f64, f64)>,
    ) -> Option<Tiklama> {
        let (rect, yanit) = ui.allocate_exact_size(boyut, egui::Sense::click());
        let painter = ui.painter_at(rect);
        painter.rect_filled(
            rect,
            egui::Rounding::same(10.0),
            egui::Color32::from_rgb(232, 236, 231),
        );

        self.bekle(ui.ctx());
        if self.yuklenen.is_none() && self.istek.is_none() && self.deneme == 0 {
            self.iste(ui.ctx());
        }
        // İstek sürerken kare yenilenmezse durum yazısı donar: çizimi sürdür.
        if self.istek.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }

        // Zemin: karo varsa gerçek harita, yoksa şematik konum çizimi.
        if let Some(yuklenen) = self
            .yuklenen
            .as_ref()
            .filter(|yuklenen| yuklenen.zoom == self.zoom)
        {
            let (mx, my) = dunya(self.merkez.0, self.merkez.1, yuklenen.zoom);
            let sol = egui::pos2(
                rect.center().x - (mx - yuklenen.sol_ust.0) as f32,
                rect.center().y - (my - yuklenen.sol_ust.1) as f32,
            );
            let doku = yuklenen.doku.id();
            let kutu = egui::Rect::from_min_size(sol, yuklenen.boyut);
            painter.image(
                doku,
                kutu,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            sema_ciz(&painter, rect, self.merkez.0, self.zoom);
        }

        // --- Pinler --------------------------------------------------------
        let (mx, my) = dunya(self.merkez.0, self.merkez.1, self.zoom);
        let ekran = |konum: (f64, f64)| {
            let (px, py) = dunya(konum.0, konum.1, self.zoom);
            egui::pos2(
                (rect.center().x + (px - mx) as f32).clamp(rect.left() + 6.0, rect.right() - 6.0),
                (rect.center().y + (py - my) as f32).clamp(rect.top() + 18.0, rect.bottom() - 22.0),
            )
        };
        // Koridor hattı: pinler arasında otoyol çizgisi.
        for ikili in cizgi.windows(2) {
            painter.line_segment(
                [ekran(ikili[0]), ekran(ikili[1])],
                egui::Stroke::new(2.5, egui::Color32::from_rgb(198, 122, 44)),
            );
        }
        let mut en_yakin: Option<(f32, String)> = None;
        for isaret in isaretler {
            let nokta = ekran(isaret.konum);
            if isaret.secili {
                pin_ciz(&painter, nokta, 9.0, egui::Color32::from_rgb(196, 30, 58));
            } else {
                pin_ciz(&painter, nokta, 6.5, egui::Color32::from_rgb(38, 74, 64));
            }
            let mesafe = nokta.distance(yanit.hover_pos().unwrap_or(egui::pos2(-999.0, -999.0)));
            if mesafe < VURMA
                && en_yakin
                    .as_ref()
                    .map(|(en, _)| mesafe < *en)
                    .unwrap_or(true)
            {
                en_yakin = Some((mesafe, isaret.anahtar.clone()));
            }
            if let Some(etiket) = &isaret.etiket {
                painter.text(
                    nokta + egui::vec2(7.0, -9.0),
                    egui::Align2::LEFT_BOTTOM,
                    etiket,
                    egui::FontId::proportional(9.5),
                    if isaret.secili {
                        egui::Color32::from_rgb(150, 20, 40)
                    } else {
                        egui::Color32::from_rgb(30, 50, 42)
                    },
                );
            }
        }
        if let Some(nokta) = taslak {
            pin_ciz(
                &painter,
                ekran(nokta),
                9.0,
                egui::Color32::from_rgb(224, 138, 20),
            );
        }

        // Alt şerit: kaynak, sayı ve durum.
        let durum = if let Some(hata) = &self.hata {
            format!("Karo sunucusuna ulaşılamadı ({hata}) · şematik konum")
        } else if self.yuklenen.is_some() {
            format!("© OpenStreetMap katkıcıları · {} pin", isaretler.len())
        } else {
            format!("Harita yükleniyor… şematik konum · {} pin", isaretler.len())
        };
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.bottom() - 18.0),
                egui::pos2(rect.right(), rect.bottom()),
            ),
            0.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 205),
        );
        painter.text(
            egui::pos2(rect.left() + 8.0, rect.bottom() - 9.0),
            egui::Align2::LEFT_CENTER,
            durum,
            egui::FontId::proportional(8.5),
            egui::Color32::from_rgb(90, 98, 94),
        );
        painter.text(
            egui::pos2(rect.right() - 8.0, rect.bottom() - 9.0),
            egui::Align2::RIGHT_CENTER,
            format!("z{}", self.zoom),
            egui::FontId::proportional(8.5),
            egui::Color32::from_rgb(90, 98, 94),
        );

        let Some(imlec) = yanit.interact_pointer_pos().filter(|_| yanit.clicked()) else {
            return None;
        };
        // Önce pin isabeti, sonra boş alan.
        for isaret in isaretler {
            let nokta = ekran(isaret.konum);
            let uzaklik = nokta.distance(imlec);
            if uzaklik <= VURMA && en_yakin.as_ref().is_none_or(|(en, _)| uzaklik < *en) {
                en_yakin = Some((uzaklik, isaret.anahtar.clone()));
            }
        }
        if let Some((_, anahtar)) = en_yakin {
            return Some(Tiklama::Isaret(anahtar));
        }
        Some(Tiklama::Konum(piksel_konum(
            self.merkez,
            self.zoom,
            rect.center(),
            imlec,
        )))
    }

    /// Tamamlanan isteği dokuya çevirir.
    fn bekle(&mut self, ctx: &egui::Context) {
        let Some(al) = self.istek.take() else {
            return;
        };
        // Askıda kalan istek (DNS/ağ) arayüzü "yükleniyor"da bırakmasın.
        if self
            .baslangic
            .is_some_and(|an| an.elapsed() > Duration::from_secs(12))
        {
            self.hata = Some("zaman aşımı".to_string());
            self.deneme += 1;
            self.baslangic = None;
            return;
        }
        match al.try_recv() {
            Ok(Ok(ham)) => {
                let doku = ctx.load_texture("pin_harita", ham.resim, egui::TextureOptions::LINEAR);
                let boyut = doku.size_vec2();
                self.yuklenen = Some(Yuklenen {
                    doku,
                    sol_ust: ham.sol_ust,
                    boyut,
                    zoom: ham.zoom,
                });
                self.hata = None;
                self.baslangic = None;
            }
            Ok(Err(hata)) => {
                self.hata = Some(hata);
                self.deneme += 1;
                self.baslangic = None;
            }
            Err(TryRecvError::Empty) => self.istek = Some(al),
            Err(TryRecvError::Disconnected) => {
                self.hata = Some("karo isteği kesildi".to_string());
                self.deneme += 1;
                self.baslangic = None;
            }
        }
    }

    /// Görünüm için karoları arka planda indirir.
    fn iste(&mut self, ctx: &egui::Context) {
        let (enlem, boylam) = self.merkez;
        let zoom = self.zoom;
        let (gonder, al) = channel();
        std::thread::spawn(move || {
            let _ = gonder.send(indir(enlem, boylam, zoom));
        });
        self.istek = Some(al);
        self.hata = None;
        self.baslangic = Some(std::time::Instant::now());
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

/// Haritaya pin çizer (gölge + damla + iç nokta).
fn pin_ciz(painter: &egui::Painter, nokta: egui::Pos2, yaricap: f32, renk: egui::Color32) {
    let bas = nokta - egui::vec2(0.0, yaricap * 1.55);
    painter.circle_filled(
        bas + egui::vec2(1.0, 1.0),
        yaricap + 0.5,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 55),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            bas + egui::vec2(-yaricap * 0.72, yaricap * 0.78),
            bas + egui::vec2(yaricap * 0.72, yaricap * 0.78),
            nokta,
        ],
        renk,
        egui::Stroke::NONE,
    ));
    painter.circle_filled(bas, yaricap, renk);
    painter.circle_filled(bas, yaricap * 0.38, egui::Color32::WHITE);
}

/// Ağ yokken gösterilen şematik konum: mesafe halkaları, artı işareti, ölçek.
fn sema_ciz(painter: &egui::Painter, rect: egui::Rect, enlem: f64, zoom: u8) {
    let cizgi = egui::Color32::from_rgb(198, 206, 198);
    let soluk = egui::Color32::from_rgb(120, 132, 120);
    for adim in 1..=6 {
        let x = rect.left() + rect.width() * adim as f32 / 7.0;
        let y = rect.top() + rect.height() * adim as f32 / 7.0;
        painter.line_segment(
            [
                egui::pos2(x, rect.top() + 18.0),
                egui::pos2(x, rect.bottom() - 18.0),
            ],
            egui::Stroke::new(
                0.6,
                egui::Color32::from_rgba_unmultiplied(198, 206, 198, 90),
            ),
        );
        painter.line_segment(
            [
                egui::pos2(rect.left() + 8.0, y),
                egui::pos2(rect.right() - 8.0, y),
            ],
            egui::Stroke::new(
                0.6,
                egui::Color32::from_rgba_unmultiplied(198, 206, 198, 90),
            ),
        );
    }
    let mpp = metre_piksel(enlem, zoom) as f32;
    for metre in [100.0_f32, 300.0, 600.0] {
        let yari = metre / mpp;
        if yari * 2.4 > rect.height() {
            continue;
        }
        painter.circle_stroke(
            rect.center(),
            yari,
            egui::Stroke::new(1.0, if metre == 100.0 { soluk } else { cizgi }),
        );
        painter.text(
            egui::pos2(rect.center().x + yari * 0.72, rect.center().y - yari * 0.72),
            egui::Align2::CENTER_CENTER,
            format!("{metre:.0} m"),
            egui::FontId::proportional(8.0),
            soluk,
        );
    }
    painter.text(
        egui::pos2(rect.right() - 18.0, rect.top() + 16.0),
        egui::Align2::CENTER_CENTER,
        "K",
        egui::FontId::proportional(9.0),
        soluk,
    );
    painter.line_segment(
        [
            egui::pos2(rect.right() - 18.0, rect.top() + 26.0),
            egui::pos2(rect.right() - 18.0, rect.top() + 40.0),
        ],
        egui::Stroke::new(1.2, soluk),
    );
    painter.text(
        egui::pos2(rect.left() + 10.0, rect.top() + 14.0),
        egui::Align2::LEFT_CENTER,
        "Şematik konum · harita karoları için internet gerekir",
        egui::FontId::proportional(8.5),
        soluk,
    );
}

/// Tıklanan ekran noktasını coğrafi konuma çevirir.
fn piksel_konum(
    merkez: (f64, f64),
    zoom: u8,
    merkez_ekran: egui::Pos2,
    imlec: egui::Pos2,
) -> (f64, f64) {
    let (px, py) = dunya(merkez.0, merkez.1, zoom);
    konum(
        px + (imlec.x - merkez_ekran.x) as f64,
        py + (imlec.y - merkez_ekran.y) as f64,
        zoom,
    )
}

/// Coğrafi konum → dünya pikseli (Web Mercator).
pub fn dunya(enlem: f64, boylam: f64, zoom: u8) -> (f64, f64) {
    let olcek = KARO as f64 * 2f64.powi(zoom as i32);
    let x = (boylam + 180.0) / 360.0 * olcek;
    // Kutuplarda Mercator sonsuza gider: sınır derece cinsinden uygulanır.
    let enlem_rad = enlem.clamp(-85.05, 85.05).to_radians();
    let y =
        (1.0 - (enlem_rad.tan() + 1.0 / enlem_rad.cos()).ln() / std::f64::consts::PI) / 2.0 * olcek;
    (x, y)
}

/// Dünya pikseli → coğrafi konum.
pub fn konum(x: f64, y: f64, zoom: u8) -> (f64, f64) {
    let olcek = KARO as f64 * 2f64.powi(zoom as i32);
    let boylam = x / olcek * 360.0 - 180.0;
    let n = std::f64::consts::PI - 2.0 * std::f64::consts::PI * y / olcek;
    let enlem = n.sinh().atan().to_degrees();
    (enlem, boylam)
}

/// Bu zoom ve enlemde bir pikselin kaç metreye denk geldiği.
pub fn metre_piksel(enlem: f64, zoom: u8) -> f64 {
    156_543.033_92 * enlem.to_radians().cos().abs().max(0.05) / 2f64.powi(zoom as i32)
}

/// Görünümü kaplayan karoları paralel indirip tek görüntüde birleştirir.
fn indir(enlem: f64, boylam: f64, zoom: u8) -> Result<Ham, String> {
    let (px, py) = dunya(enlem, boylam, zoom);
    let sol = px - YARI;
    let ust = py - YARI;
    let x0 = (sol / KARO as f64).floor() as i64;
    let y0 = (ust / KARO as f64).floor() as i64;
    let x1 = ((sol + YARI * 2.0) / KARO as f64).floor() as i64;
    let y1 = ((ust + YARI * 2.0) / KARO as f64).floor() as i64;
    if (x1 - x0 + 1) * (y1 - y0 + 1) > 16 {
        return Err("görünüm için fazla karo gerekiyor".to_string());
    }
    let adet = 2i64.pow(zoom as u32);
    let mut istekler: Vec<(i64, i64, String)> = Vec::new();
    for y in y0..=y1 {
        if !(0..adet).contains(&y) {
            continue;
        }
        for x in x0..=x1 {
            let xk = x.rem_euclid(adet);
            istekler.push((
                x,
                y,
                format!("https://tile.openstreetmap.org/{zoom}/{xk}/{y}.png"),
            ));
        }
    }
    // Karolar paralel indirilir: tek engelli karo tüm görünümü bekletmez.
    let sonuclar: Vec<(i64, i64, Vec<u8>)> = std::thread::scope(|kapsam| {
        let tutamaclar: Vec<_> = istekler
            .into_iter()
            .map(|(x, y, url)| kapsam.spawn(move || karo_indir(&url).map(|bayt| (x, y, bayt))))
            .collect();
        let mut cikti = Vec::new();
        for tutamac in tutamaclar {
            if let Ok(Ok(karo)) = tutamac.join() {
                cikti.push(karo);
            }
        }
        cikti
    });
    if sonuclar.is_empty() {
        return Err("ağ hatası".to_string());
    }
    let genis = ((x1 - x0 + 1) * KARO as i64) as usize;
    let yuksek = ((y1 - y0 + 1) * KARO as i64) as usize;
    let mut tuval = vec![235u8; genis * yuksek * 4];
    for (x, y, bayt) in sonuclar {
        let Ok(resim) = image::load_from_memory(&bayt) else {
            continue;
        };
        let resim = resim.to_rgba8();
        if resim.width() != KARO || resim.height() != KARO {
            continue;
        }
        let ofs_x = ((x - x0) * KARO as i64) as usize;
        let ofs_y = ((y - y0) * KARO as i64) as usize;
        let kaynak = resim.as_raw();
        for satir in 0..KARO as usize {
            let hedef = ((ofs_y + satir) * genis + ofs_x) * 4;
            let kaynak_bas = satir * KARO as usize * 4;
            tuval[hedef..hedef + KARO as usize * 4]
                .copy_from_slice(&kaynak[kaynak_bas..kaynak_bas + KARO as usize * 4]);
        }
    }
    Ok(Ham {
        resim: egui::ColorImage::from_rgba_unmultiplied([genis, yuksek], &tuval),
        sol_ust: (x0 as f64 * KARO as f64, y0 as f64 * KARO as f64),
        zoom,
    })
}

fn karo_indir(url: &str) -> Result<Vec<u8>, String> {
    let yanit = ureq::get(url)
        .set("User-Agent", TANITICI)
        .timeout(Duration::from_secs(5))
        .call()
        .map_err(|hata| format!("{hata}"))?;
    let mut bayt = Vec::new();
    use std::io::Read;
    yanit
        .into_reader()
        .take(1_500_000)
        .read_to_end(&mut bayt)
        .map_err(|hata| format!("{hata}"))?;
    Ok(bayt)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn dunya_ve_konum_tersine_doner() {
        for (enlem, boylam) in [(39.004814, 28.116757), (38.6123, 27.4321), (0.0, 0.0)] {
            let (x, y) = dunya(enlem, boylam, 16);
            let (e2, b2) = konum(x, y, 16);
            assert!((enlem - e2).abs() < 1e-6, "enlem {enlem} → {e2}");
            assert!((boylam - b2).abs() < 1e-6, "boylam {boylam} → {b2}");
        }
    }

    #[test]
    fn zoom_buyudukce_olcek_artar() {
        let (x1, _) = dunya(39.0, 28.0, 14);
        let (x2, _) = dunya(39.0, 28.0, 15);
        assert!((x2 - 2.0 * x1).abs() < 1.0);
    }

    #[test]
    fn kutuplar_sinirlanir() {
        let (_, y) = dunya(89.9, 0.0, 16);
        assert!(y.is_finite() && y >= 0.0);
    }

    #[test]
    fn olcek_metre_piksel_makul() {
        // 39. enlem, z16: yaklaşık 1,9 m/piksel.
        let mpp = metre_piksel(39.0, 16);
        assert!((1.5..2.4).contains(&mpp), "mpp {mpp}");
        assert!(metre_piksel(39.0, 17) < mpp);
    }

    #[test]
    fn sigdir_tum_noktalari_kaplar() {
        let mut harita = Harita::default();
        let noktalar = [
            (38.732307, 27.570621),
            (38.7319147, 27.5708628),
            (38.7357627, 27.5638351),
            (38.7292501, 27.5784208),
        ];
        harita.sigdir(&noktalar);
        let (x0, y0) = dunya(38.7292501, 27.5638351, harita.zoom);
        let (x1, y1) = dunya(38.7357627, 27.5784208, harita.zoom);
        assert!(x1 - x0 <= 380.0, "genişlik {}", x1 - x0);
        assert!(y1 - y0 <= 220.0, "yükseklik {}", y1 - y0);
        assert!(harita.zoom >= EN_AZ_ZOOM);
        assert!((harita.merkez.0 - 38.7325).abs() < 0.01);
    }

    #[test]
    fn tek_noktada_yakin_zoom() {
        let mut harita = Harita::default();
        harita.sigdir(&[(38.732307, 27.570621)]);
        assert_eq!(harita.zoom, 16);
        assert_eq!(harita.merkez, (38.732307, 27.570621));
    }

    #[test]
    fn piksel_konum_merkezi_korur() {
        let merkez = egui::pos2(100.0, 100.0);
        let (enlem, boylam) = piksel_konum((39.0, 28.0), 16, merkez, merkez);
        assert!((enlem - 39.0).abs() < 1e-6);
        assert!((boylam - 28.0).abs() < 1e-6);
    }

    #[test]
    fn uzun_koridor_ekrana_sigar() {
        let mut harita = Harita::default();
        harita.sigdir_alana(&[(40.9, 29.19), (38.16, 27.36)], 960.0, 340.0);
        assert!(harita.zoom() <= 8, "koridor zoom'u: {}", harita.zoom());
        let (x0, y0) = dunya(40.9, 27.36, harita.zoom());
        let (x1, y1) = dunya(38.16, 29.19, harita.zoom());
        assert!(
            ((x1 - x0) as f32) <= 960.0 && ((y1 - y0) as f32) <= 340.0,
            "zoom {} için alan taşıyor",
            harita.zoom()
        );
    }
}
