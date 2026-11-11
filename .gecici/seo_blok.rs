impl LeadApp {
    /// Katman 2 — Google İşletme linki, koordinat çözümü, harita işareti.
    fn seo_alani_ekrani(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ARKA_PLAN)
                    .inner_margin(egui::Margin::symmetric(22.0, 18.0)),
            )
            .show(ctx, |ui| {
                bolum_etiketi(ui, "2 · SEO ÇALIŞMA ALANI");
                ui.label(
                    egui::RichText::new(
                        "Google İşletme/Maps linkini yapıştırın: koordinat çözülür, haritada işaretlenir ve SEO motoru aynı bağlamla tetiklenir.",
                    )
                    .size(10.5)
                    .color(DIM),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let genislik = (ui.available_width() - 270.0).max(220.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.seo_link)
                            .hint_text("https://www.google.com/maps/place/...")
                            .desired_width(genislik),
                    );
                    if birincil_buton(ui, "KONUMU ÇÖZ", true, Some(egui::vec2(112.0, 30.0)))
                        .clicked()
                    {
                        match crate::isletme_linki::coz(&self.seo_link) {
                            Ok(konum) => {
                                self.logs.push(format!(
                                    "[✓] Konum çözüldü: {} · {}",
                                    konum.ad,
                                    konum.koordinat()
                                ));
                                self.seo_konum = Some(konum);
                            }
                            Err(hata) => {
                                self.logs.push(format!("[!] {hata}"));
                                self.seo_konum = None;
                            }
                        }
                    }
                    if ikincil_buton(ui, "SEO MOTORU", true, Some(egui::vec2(120.0, 30.0)))
                        .clicked()
                    {
                        self.seo_calistir();
                        self.katman = Katman::SeoOnizleme;
                    }
                });
                ui.add_space(12.0);
                match self.seo_konum.clone() {
                    Some(konum) => {
                        let boyut = egui::vec2(ui.available_width(), 300.0);
                        let (rect, _) = ui.allocate_exact_size(boyut, egui::Sense::hover());
                        harita_ciz(ui, rect, &konum);
                        ui.add_space(6.0);
                        let place = if konum.place_id.is_empty() {
                            "—"
                        } else {
                            konum.place_id.as_str()
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} · place_id: {place}",
                                konum.ad,
                                konum.koordinat()
                            ))
                            .size(10.0)
                            .color(METIN),
                        );
                    }
                    None => {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("Harita: henüz konum çözülmedi.")
                                    .size(11.0)
                                    .color(DIM),
                            );
                        });
                    }
                }
            });
    }

    /// Katman 3 — üretilen SEO sayfasının meta/şema önizlemesi ve onayı.
    fn seo_onizleme_ekrani(&mut self, ctx: &egui::Context) {
        let metin = std::fs::read_to_string(crate::web_veri::VERI_YOLU).unwrap_or_default();
        let veri: Option<serde_json::Value> = serde_json::from_str(&metin).ok();
        let site = veri
            .as_ref()
            .and_then(|v| v.get("siteler"))
            .and_then(|s| s.as_array())
            .and_then(|s| s.first())
            .cloned();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ARKA_PLAN)
                    .inner_margin(egui::Margin::symmetric(22.0, 18.0)),
            )
            .show(ctx, |ui| {
                bolum_etiketi(ui, "3 · SEO ÖNİZLEME");
                ui.label(
                    egui::RichText::new(
                        "Motorun ürettiği başlık, açıklama, canonical, Open Graph ve JSON-LD; onaylanan sayfa 4 Gelen'e aktarılır.",
                    )
                    .size(10.5)
                    .color(DIM),
                );
                ui.add_space(10.0);
                if let Some(konum) = self.seo_konum.clone() {
                    let boyut = egui::vec2(ui.available_width(), 130.0);
                    let (rect, _) = ui.allocate_exact_size(boyut, egui::Sense::hover());
                    harita_ciz(ui, rect, &konum);
                    ui.add_space(8.0);
                }
                let Some(site) = site else {
                    ui.label(
                        egui::RichText::new("Yayında sayfa yok: 2. sekmede SEO motorunu çalıştırın.")
                            .size(11.0)
                            .color(TURUNCU),
                    );
                    return;
                };
                let alan = |ad: &str, deger: &str, sinir: usize| {
                    let uzunluk = deger.chars().count();
                    let renk = if uzunluk == 0 || uzunluk > sinir {
                        KIRMIZI
                    } else {
                        YESIL
                    };
                    ui.label(
                        egui::RichText::new(format!("{ad} ({uzunluk}/{sinir}): {deger}"))
                            .size(10.5)
                            .color(renk),
                    );
                };
                let metin_alani = |ad: &str| {
                    site.get(ad)
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string()
                };
                let slug = metin_alani("slug");
                alan("title", &metin_alani("baslik"), 60);
                alan("description", &metin_alani("aciklama"), 160);
                alan("canonical", &format!("/s/{slug}"), 120);
                alan("JSON-LD @type", &metin_alani("sema"), 40);
                let medya = site
                    .get("medya")
                    .and_then(|m| m.as_array())
                    .map(|m| m.len())
                    .unwrap_or(0);
                alan("medya", &format!("{medya} dosya"), 40);
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("SSR durumu: {}", ssr_kodu(&slug)))
                        .size(10.0)
                        .color(MAVI),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if birincil_buton(
                        ui,
                        "ÖNİZLEMEYİ ONAYLA · 4 GELEN",
                        true,
                        Some(egui::vec2(260.0, 34.0)),
                    )
                    .clicked()
                    {
                        if !self.seo_onaylanan.contains(&slug) {
                            self.seo_onaylanan.push(slug.clone());
                        }
                        self.logs.push(format!(
                            "[✓] {slug} SEO önizlemesinden onaylandı; Gelen Kutusu'na aktarıldı."
                        ));
                        self.katman = Katman::GelenKutusu;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("onaylanan: {}", self.seo_onaylanan.len()))
                            .size(10.0)
                            .color(DIM),
                    );
                });
            });
    }
}

/// Koordinatı harita ızgarası üzerinde işaretler (döşeme indirmeyen, çevrimdışı
/// çalışan hafif harita paneli).
fn harita_ciz(ui: &mut egui::Ui, rect: egui::Rect, konum: &crate::isletme_linki::Konum) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        egui::Rounding::same(12.0),
        egui::Color32::from_rgb(16, 24, 34),
    );
    let adim = 32.0;
    let mut x = rect.left() + adim;
    while x < rect.right() {
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, KENAR),
        );
        x += adim;
    }
    let mut y = rect.top() + adim;
    while y < rect.bottom() {
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, KENAR),
        );
        y += adim;
    }
    let merkez = rect.center();
    painter.circle_stroke(merkez, 22.0, egui::Stroke::new(1.0, VURGU));
    painter.circle_filled(merkez, 8.0, VURGU);
    painter.circle_stroke(merkez, 8.0, egui::Stroke::new(2.0, KOYU_YAZI));
    painter.text(
        egui::pos2(rect.left() + 12.0, rect.top() + 10.0),
        egui::Align2::LEFT_TOP,
        "HARİTA · KONUM İŞARETİ",
        baslik_font(9.0),
        egui::Color32::from_rgb(229, 169, 59),
    );
    painter.text(
        egui::pos2(merkez.x, merkez.y + 30.0),
        egui::Align2::CENTER_TOP,
        konum.koordinat(),
        kod_font(10.0),
        METIN,
    );
    painter.text(
        egui::pos2(merkez.x, merkez.y + 48.0),
        egui::Align2::CENTER_TOP,
        &konum.ad,
        govde_font(10.5),
        DIM,
    );
}

/// Yerel SSR sunucusundan tek satırlık durum kodu okur.
fn ssr_kodu(slug: &str) -> String {
    use std::io::{Read, Write};
    let Ok(mut akis) = std::net::TcpStream::connect(("127.0.0.1", 8090)) else {
        return "sunucu kapalı (8090)".to_string();
    };
    let _ = akis.set_read_timeout(Some(std::time::Duration::from_millis(600)));
    let istek = format!("GET /s/{slug} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    if akis.write_all(istek.as_bytes()).is_err() {
        return "istek yazılamadı".to_string();
    }
    let mut tampon = [0u8; 64];
    match akis.read(&mut tampon) {
        Ok(n) if n > 0 => String::from_utf8_lossy(&tampon[..n])
            .lines()
            .next()
            .unwrap_or("")
            .to_string(),
        _ => "yanıt yok".to_string(),
    }
}
