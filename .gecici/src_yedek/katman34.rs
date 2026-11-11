//! Katman 4 (domain + temiz derleme + yayın) ve Katman 5 (gelen kutusu).
//!
//! Katman 4 müşteriye gidecek *son* dosyayı üretir: demo şeridi, taslak notu,
//! 12 saatlik sayaç, örnek yorumlar ve stok kareler sayfadan çıkar. Katman 5
//! müşterinin yanıtını gösterir; yanıt yerel köprüden (127.0.0.1) gelir.
//! İki katman da dış tarayıcı açmaz.

use crate::*;
use eframe::egui;

/// Müşteri sürümünden çıkarılan demo izleri (arayüzde liste olarak görünür).
const DEMO_IZLERI: [(&str, &str); 5] = [
    ("Taslak demo şeridi", "sayfanın en üstündeki tanıtım bandı"),
    (
        "Altbilgi taslak notu",
        "\"Bu taslak … için hazırlandı\" cümlesi",
    ),
    (
        "12 saatlik demo sayacı",
        "süre dolunca sayfayı kapatan katman",
    ),
    (
        "Örnek yorumlar",
        "müşteri yorumu yerine konan yer tutucular",
    ),
    (
        "Stok galeri kareleri",
        "işletmenin fotoğrafı olmayan örnek kareler",
    ),
];

impl LeadApp {
    // -----------------------------------------------------------------------
    // Katman 5 — gelen kutusu
    // -----------------------------------------------------------------------
    pub(crate) fn gelen_ekrani(&mut self, ctx: &egui::Context) {
        let mut secim: Option<usize> = None;
        let mut asama_degis: Option<(Vec<usize>, gelen::Asama)> = None;
        let mut niyet_degis: Option<(Vec<usize>, gelen::Niyet)> = None;
        let mut elle_ekle = false;
        let mut yanitla = false;
        let kanallar = self.gelen.kanallar();
        let (_, okunmamis, sicak) = self.gelen.sayilar();

        egui::SidePanel::left("gelen_kanallar")
            .resizable(false)
            .exact_width(298.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14.0, 14.0)),
            )
            .show(ctx, |ui| {
                bolum_etiketi(ui, "WHATSAPP");
                ui.horizontal(|ui| {
                    pill(ui, &format!("{okunmamis} okunmamış"), if okunmamis > 0 { MAVI } else { DIM });
                    pill(ui, &format!("{sicak} sıcak"), if sicak > 0 { YESIL } else { DIM });
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .id_source("gelen_kanal_listesi")
                    .auto_shrink([false, false])
                    .max_height(ui.available_height() - 150.0)
                    .show(ui, |ui| {
                        if kanallar.is_empty() {
                            ui.label(
                                egui::RichText::new(
                                    "Henüz mesaj yok. Müşteri demo linkinden yazdığında burada görünür.",
                                )
                                .size(10.0)
                                .color(DIM),
                            );
                        }
                        for (sira, kanal) in kanallar.iter().enumerate() {
                            let secili = self.gelen_secili == Some(sira);
                            let (renk, isaret) = match kanal.niyet {
                                gelen::Niyet::Sicak => (YESIL, "sıcak"),
                                gelen::Niyet::Fiyat => (KEHRIBAR, "fiyat"),
                                gelen::Niyet::Beklemede => (TURUNCU, "beklemede"),
                                gelen::Niyet::Ret => (KIRMIZI, "ret"),
                                gelen::Niyet::Bilinmiyor => (DIM, "belirsiz"),
                            };
                            let onizleme = kanal
                                .satirlar
                                .last()
                                .and_then(|indeks| self.gelen.kayitlar().get(*indeks))
                                .map(|kayit| kayit.ozet())
                                .unwrap_or_default();
                            let cevap = egui::Frame::none()
                                .fill(if secili {
                                    ton(VURGU, 0.12)
                                } else {
                                    egui::Color32::TRANSPARENT
                                })
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(kisalt_metin(
                                                ui,
                                                &kanal.ad,
                                                ui.available_width() - 60.0,
                                                11.0,
                                            ))
                                            .font(baslik_font(11.0))
                                            .color(METIN),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if kanal.okunmamis > 0 {
                                                    pill(ui, &kanal.okunmamis.to_string(), MAVI);
                                                }
                                            },
                                        );
                                    });
                                    ui.label(
                                        egui::RichText::new(onizleme).size(9.0).color(DIM),
                                    );
                                    ui.horizontal(|ui| {
                                        pill(ui, isaret, renk);
                                        ui.label(
                                            egui::RichText::new(&kanal.son_zaman)
                                                .size(8.5)
                                                .color(DIM),
                                        );
                                    });
                                });
                            // Tıklama alanı yalnız bu satırın dikdörtgenidir.
                            let yanit = ui.interact(
                                cevap.response.rect,
                                egui::Id::new(("kanal", sira)),
                                egui::Sense::click(),
                            );
                            if yanit.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if yanit.clicked() {
                                secim = Some(sira);
                            }
                            ui.add_space(3.0);
                        }
                    });

                if hayalet_buton(ui, &format!("{}Elle mesaj / yanıt ekle", if self.elle_form { "▾ " } else { "▸ " })).clicked() {
                    self.elle_form = !self.elle_form;
                }
                if self.elle_form {
                    ui.add_space(6.0);
                    metin_kutusu(ui, &mut self.elle_telefon, "telefon (ör. 0555…)");
                    ui.add_space(4.0);
                    metin_kutusu(ui, &mut self.elle_ad, "ad (isteğe bağlı)");
                    ui.add_space(4.0);
                    metin_alani(ui, "Gelen mesaj", &mut self.elle_metin, "Müşterinin yazdığı metin", 2);
                    if ikincil_buton(ui, "Kutuya ekle", !self.elle_metin.trim().is_empty(), Some(egui::vec2(ui.available_width(), 28.0))).clicked() {
                        elle_ekle = true;
                    }
                }
            });

        // --- Sağ: köprü durumu + yanıt -------------------------------------
        egui::SidePanel::right("gelen_kopru")
            .resizable(false)
            .exact_width(304.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14.0, 14.0)),
            )
            .show(ctx, |ui| {
                bolum_etiketi(ui, "YEREL KÖPRÜ");
                ui.add_space(4.0);
                pill(
                    ui,
                    if self.kopru.calisiyor { "oturum açık" } else { "oturum kapalı" },
                    if self.kopru.calisiyor { YESIL } else { TURUNCU },
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(&self.kopru.mesaj)
                        .font(kod_font(9.0))
                        .color(DIM),
                );
                ui.add_space(8.0);
                bolum_etiketi(ui, "UÇLAR");
                for satir in [
                    format!("POST /gelen  · 127.0.0.1:{}", self.gelen.port()),
                    format!("POST /gonder · 127.0.0.1:{}", self.gelen.port()),
                    "GET  /durum".to_string(),
                ] {
                    ui.label(egui::RichText::new(satir).font(kod_font(8.5)).color(SOLUK));
                }
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        "Köprü mesajı buraya bırakır; uygulama dış tarayıcı açmaz. Oturum yoksa metin panoya kopyalanır.",
                    )
                    .size(9.0)
                    .color(DIM),
                );
                ui.add_space(12.0);
                self.ai_paneli(ui);
                bolum_etiketi(ui, "YANIT");
                let hedef = self
                    .gelen_secili
                    .and_then(|sira| kanallar.get(sira))
                    .map(|kanal| (kanal.telefon.clone(), kanal.ad.clone()));
                match &hedef {
                    Some((telefon, ad)) => {
                        ui.label(
                            egui::RichText::new(format!("{ad} · {telefon}"))
                                .size(9.5)
                                .color(SOLUK),
                        );
                    }
                    None => {
                        ui.label(
                            egui::RichText::new("Soldan bir yazışma seçin.")
                                .size(9.5)
                                .color(DIM),
                        );
                    }
                }
                ui.add_space(4.0);
                egui::Frame::none()
                    .fill(GIRINTI)
                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                    .rounding(egui::Rounding::same(9.0))
                    .inner_margin(egui::Margin::symmetric(9.0, 6.0))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.gelen_yanit)
                                .frame(false)
                                .desired_rows(4)
                                .hint_text(
                                    egui::RichText::new("Yanıt metni…").size(11.0).color(DIM),
                                )
                                .font(govde_font(11.5))
                                .desired_width(f32::INFINITY),
                        );
                    });
                ui.add_space(6.0);
                let hazir = hedef.is_some() && !self.gelen_yanit.trim().is_empty();
                if birincil_buton(ui, "Köprüden gönder", hazir, Some(egui::vec2(ui.available_width(), 30.0))).clicked() {
                    yanitla = true;
                }
                ui.add_space(4.0);
                if hayalet_buton(ui, "Metni panoya kopyala").clicked() {
                    let metin = self.gelen_yanit.clone();
                    ctx.output_mut(|cikti| cikti.copied_text = metin);
                }
            });

        // --- Orta: yazışma --------------------------------------------------
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ARKA_PLAN)
                    .inner_margin(egui::Margin::symmetric(18.0, 14.0)),
            )
            .show(ctx, |ui| {
                let Some(sira) = self.gelen_secili else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.label(
                            egui::RichText::new("Soldan bir yazışma seçin.")
                                .size(12.0)
                                .color(DIM),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(
                                "Gelen mesaj yoksa: demo linkini paylaştıktan sonra müşterinin yanıtı buraya düşer.",
                            )
                            .size(10.0)
                            .color(DIM),
                        );
                    });
                    return;
                };
                let Some(kanal) = kanallar.get(sira) else { return; };

                ui.horizontal(|ui| {
                    avatar(ui, &kanal.ad, MAVI);
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&kanal.ad)
                                .font(baslik_font(14.0))
                                .color(METIN),
                        );
                        ui.label(
                            egui::RichText::new(&kanal.telefon)
                                .size(9.5)
                                .color(DIM),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if birincil_buton(ui, "Satış kapandı", true, Some(egui::vec2(130.0, 30.0))).clicked() {
                            asama_degis = Some((kanal.satirlar.clone(), gelen::Asama::SatisKapandi));
                        }
                        ui.add_space(6.0);
                        if ikincil_buton(ui, "Demo atıldı", true, Some(egui::vec2(118.0, 30.0))).clicked() {
                            asama_degis = Some((kanal.satirlar.clone(), gelen::Asama::DemoAtildi));
                        }
                    });
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(egui::RichText::new("NİYET").size(8.5).color(DIM));
                    for niyet in gelen::Niyet::HEPSI {
                        if filtre_cipi(ui, niyet.etiket(), kanal.niyet == niyet).clicked()
                            && kanal.niyet != niyet
                        {
                            niyet_degis = Some((kanal.satirlar.clone(), niyet));
                        }
                    }
                });
                ui.add_space(8.0);

                let yukseklik = (ui.available_height() - 8.0).max(180.0);
                egui::ScrollArea::vertical()
                    .id_source("gelen_yazisma")
                    .stick_to_bottom(true)
                    .auto_shrink([false, false])
                    .max_height(yukseklik)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for indeks in &kanal.satirlar {
                            let Some(kayit) = self.gelen.kayitlar().get(*indeks) else {
                                continue;
                            };
                            let gelen_mi = kayit.yon == gelen::Yon::Gelen;
                            let dolgu = if gelen_mi { KART } else { ton(VURGU, 0.14) };
                            let duzen = if gelen_mi {
                                egui::Layout::left_to_right(egui::Align::Min)
                            } else {
                                egui::Layout::right_to_left(egui::Align::Min)
                            };
                            ui.with_layout(duzen, |ui| {
                                egui::Frame::none()
                                    .fill(dolgu)
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(10.0))
                                    .inner_margin(egui::Margin::symmetric(11.0, 8.0))
                                    .show(ui, |ui| {
                                        ui.set_max_width(340.0);
                                        ui.label(
                                            egui::RichText::new(&kayit.metin)
                                                .size(11.5)
                                                .color(METIN),
                                        );
                                        ui.add_space(2.0);
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&kayit.zaman)
                                                    .size(8.0)
                                                    .color(DIM),
                                            );
                                            if gelen_mi {
                                                pill(ui, kayit.yon.etiket(), DIM);
                                            pill(ui, kayit.niyet.etiket(), MAVI);
                                            }
                                        });
                                    });
                            });
                            ui.add_space(5.0);
                        }
                    });
            });

        // --- Panel kapanışından sonra durum değişiklikleri ------------------
        if let Some(sira) = secim {
            self.gelen_secili = Some(sira);
            if let Some(kanal) = kanallar.get(sira) {
                let satirlar = kanal.satirlar.clone();
                self.gelen.okundu_isaretle(&satirlar);
                self.gelen.kaydet();
            }
        }
        if let Some((satirlar, asama)) = asama_degis {
            self.gelen.asama_ayarla(&satirlar, asama);
            self.gelen.kaydet();
            self.logs.push(format!("[i] Durum: {}.", asama.etiket()));
        }
        if let Some((satirlar, niyet)) = niyet_degis {
            self.gelen.niyet_ayarla(&satirlar, niyet);
            self.gelen.kaydet();
        }
        if elle_ekle {
            let kayit = gelen::Kayit::yeni(
                &self.elle_telefon,
                &self.elle_ad,
                &self.elle_metin,
                gelen::Yon::Gelen,
                "elle",
            );
            self.logs.push(format!(
                "[✓] Elle mesaj eklendi · {} · {}",
                kayit.baslik(),
                kayit.niyet.etiket()
            ));
            self.gelen.ekle(kayit);
            self.gelen.kaydet();
            self.elle_metin.clear();
            self.elle_form = false;
            self.gelen_secili = Some(0);
        }
        if yanitla {
            if let Some((telefon, ad)) = self
                .gelen_secili
                .and_then(|sira| kanallar.get(sira))
                .map(|kanal| (kanal.telefon.clone(), kanal.ad.clone()))
            {
                self.yanit_gonder(ctx, &telefon, &ad);
            }
        }
    }

    /// Yanıtı köprüye bırakır; köprü yoksa panoya kopyalar.
    fn yanit_gonder(&mut self, ctx: &egui::Context, telefon: &str, ad: &str) {
        let metin = self.gelen_yanit.trim().to_string();
        if metin.is_empty() {
            return;
        }
        let port = self.gelen.port();
        match gelen::gonder(port, telefon, ad, &metin) {
            Ok(()) => {
                self.gelen.ekle(gelen::Kayit::yeni(
                    telefon,
                    ad,
                    &metin,
                    gelen::Yon::Giden,
                    "uygulama",
                ));
                self.gelen.kaydet();
                self.gelen_yanit.clear();
                self.kopru.calisiyor = true;
                self.logs.push("[✓] Yanıt köprüye bırakıldı.".to_string());
            }
            Err(hata) => {
                self.kopru.calisiyor = false;
                ctx.output_mut(|cikti| cikti.copied_text = metin.clone());
                self.logs
                    .push(format!("[!] Köprü yok ({hata}); yanıt panoya kopyalandı."));
            }
        }
    }
}

/// Müşteriye gidecek son sayfa: demo izleri ayıklanmış hâli.
/// (Silinen iz sayısı, izleme sayısı.)
pub(crate) fn musteri_sayfasi(lead: &BusinessLead, gonderen: &str) -> (String, usize) {
    let (temiz, iz_sayisi) = demo_temizle(&demo_html_sec(lead, gonderen, true));
    // SEO/şema katmanı kaldırıldı: sayfa yalnız demo izleri ayıklanmış hâliyle döner.
    (temiz, iz_sayisi)
}

fn buyuk_sayi(ui: &mut egui::Ui, etiket: &str, deger: usize, renk: egui::Color32) {
    egui::Frame::none()
        .fill(GIRINTI)
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(11.0, 9.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(etiket)
                    .font(baslik_font(8.0))
                    .color(DIM),
            );
            ui.label(
                egui::RichText::new(deger.to_string())
                    .font(baslik_font(22.0))
                    .color(renk),
            );
        });
}

/// Demo sayfasındaki tanıtım izlerini ayıklar; (temiz metin, silinen iz sayısı).
///
/// Kesimler birebir üreticinin yazdığı işaretlere dayanır: serit div'i, altbilgi
/// notu, "süresi doldu" katmanı ve 12 saatlik sayaç betiği. Metin bulunamazsa
/// sayfa olduğu gibi kalır (bozuk HTML üretmektense dokunmamak yeğdir).
pub(crate) fn demo_temizle(html: &str) -> (String, usize) {
    let mut metin = html.to_string();
    let mut sayi = 0usize;

    let kes = |metin: &mut String, bas_izi: &str, bitis_izi: &str, sayi: &mut usize| {
        let Some(bas) = metin.find(bas_izi) else {
            return;
        };
        let Some(bitis) = metin[bas..].find(bitis_izi) else {
            return;
        };
        let son = bas + bitis + bitis_izi.len();
        let son = if metin[son..].starts_with('\n') {
            son + 1
        } else {
            son
        };
        metin.replace_range(bas..son, "");
        *sayi += 1;
    };

    kes(
        &mut metin,
        "<div class=\"serit\">Taslak demo · ",
        "</div>",
        &mut sayi,
    );
    kes(
        &mut metin,
        "<p class=\"alt-not\">Bu taslak ",
        "</p>",
        &mut sayi,
    );
    kes(
        &mut metin,
        "<div class=\"suresi-doldu\"",
        "</div>",
        &mut sayi,
    );
    kes(
        &mut metin,
        "<script>\n(function(){\n  var a='demo_",
        "</script>",
        &mut sayi,
    );

    // Demo sayfası arama motorlarına kapalı gelir (noindex). Canlıya çıkan
    // müşteri sitesi dizine girmelidir; aksi hâlde site haritada hiç görünmez.
    let noindex = "<meta name=\"robots\" content=\"noindex\">";
    if metin.contains(noindex) {
        metin = metin.replace(noindex, "<meta name=\"robots\" content=\"index,follow\">");
    }

    (metin, sayi)
}

/// Amber tonu (Katman 5'in fiyat niyeti etiketinde kullanılır).
pub(crate) const KEHRIBAR: egui::Color32 = egui::Color32::from_rgb(176, 122, 20);

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn demo_izleri_ayiklanir() {
        let ham = "<html>\n<div class=\"serit\">Taslak demo · X için hazırlandı · içerik taslaktır</div>\n\
<p class=\"alt-not\">Bu taslak +90555 tarafından X için hazırlandı.</p>\n\
<div class=\"suresi-doldu\" id=\"suresi-doldu\" hidden>\n<p>Bu demonun süresi doldu.</p>\n</div>\n\
<script>\n(function(){\n  var a='demo_x', b=0;\n})();\n</script>\n</body>\n</html>\n";
        let (temiz, sayi) = demo_temizle(ham);
        assert_eq!(sayi, 4);
        assert!(!temiz.contains("Taslak demo"));
        assert!(!temiz.contains("alt-not"));
        assert!(!temiz.contains("suresi-doldu"));
        assert!(!temiz.contains("demo_x"));
        // Sayfa iskeleti korunur.
        assert!(temiz.contains("</body>"));
        assert!(temiz.contains("</html>"));
    }

    #[test]
    fn iz_yoksa_sayfa_degismez() {
        let sade = "<html><body><h1>Site</h1></body></html>";
        let (temiz, sayi) = demo_temizle(sade);
        assert_eq!(sayi, 0);
        assert_eq!(temiz, sade);
    }
    #[test]
    fn canli_surum_arama_motoruna_acilir() {
        let ham = "<head><meta name=\"robots\" content=\"noindex\">
<div class=\"serit\">Taslak demo · X için hazırlandı · içerik taslaktır</div>
</head>";
        let (temiz, sayi) = demo_temizle(ham);
        assert_eq!(sayi, 1);
        assert!(!temiz.contains("noindex"), "canlı sitede noindex kalmamalı");
        assert!(temiz.contains("index,follow"));
    }
}
