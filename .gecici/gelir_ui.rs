impl LeadApp {
    /// Gelir katmanının tamamı: pano, müşteri listesi ve hizmet seçimi.
    ///
    /// Tek bir `CentralPanel` içinde iki sütun çizilir; böylece katman
    /// değişiminde panel sırası sorunu çıkmaz.
    pub(crate) fn gelir_ekrani(&mut self, ctx: &egui::Context) {
        let (mut secilen, mut hedef_degis) = (None, None);
        let mut paket_sec: Option<String> = None;
        let mut paket_birak = false;
        let mut hizmet_degis: Option<(String, bool)> = None;
        let mut tek_sefer_degis: Option<(String, bool)> = None;
        let mut teklif_iste = false;
        let mut fatura_iste = false;
        let mut kopyala_iste = false;

        let mrr = self.gelir.mrr();
        let abone = self.gelir.abone_sayisi();
        let hedef_sira = self.gelir_hedef.min(HEDEFLER.len() - 1);
        let (hedef, hedef_etiket) = HEDEFLER[hedef_sira];
        let oran = (mrr as f32 / hedef as f32).clamp(0.0, 1.0);
        let ortalama = if abone > 0 { mrr / abone as u32 } else { 5_000 };
        let eksik = hedef.saturating_sub(mrr);
        let eksik_abone = if ortalama > 0 {
            (eksik + ortalama - 1) / ortalama
        } else {
            0
        };

        let secili_anahtar = self.gelir_secili.clone();
        let musteri = secili_anahtar.as_ref().and_then(|anahtar| {
            self.defter
                .kayit(anahtar)
                .map(|kayit| (kayit.ad.clone(), kayit.telefon.clone()))
        });
        let gelir_kaydi = secili_anahtar
            .as_ref()
            .and_then(|anahtar| self.gelir.kayit(anahtar).cloned())
            .unwrap_or_default();

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ARKA_PLAN)
                    .inner_margin(egui::Margin::symmetric(18.0, 14.0)),
            )
            .show(ctx, |ui| {
                // --- Pano ---------------------------------------------------
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        bolum_etiketi(ui, "GELİR");
                        ui.label(
                            egui::RichText::new(format!("{} TL/ay", fiyat_yaz(mrr)))
                                .font(baslik_font(23.0))
                                .color(METIN),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{abone} abone · ortalama {} TL · hedef {} ({})",
                                fiyat_yaz(ortalama),
                                fiyat_yaz(hedef),
                                hedef_etiket
                            ))
                            .size(10.0)
                            .color(DIM),
                        );
                    });
                    ui.add_space(24.0);
                    ui.vertical(|ui| {
                        ui.add_space(10.0);
                        let genislik = (ui.available_width() - 300.0).max(180.0);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(genislik, 8.0), egui::Sense::hover());
                        let painter = ui.painter();
                        painter.rect_filled(rect, egui::Rounding::same(4.0), GIRINTI);
                        if oran > 0.0 {
                            let dolu = egui::Rect::from_min_size(
                                rect.min,
                                egui::vec2((rect.width() * oran).max(6.0), rect.height()),
                            );
                            painter.rect_filled(dolu, egui::Rounding::same(4.0), VURGU);
                        }
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "%{} · hedefe {} TL · yaklaşık {} abone daha",
                                (oran * 100.0).round() as i32,
                                fiyat_yaz(eksik),
                                eksik_abone
                            ))
                            .size(10.5)
                            .color(SOLUK),
                        );
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("HEDEF").size(9.0).color(DIM));
                            for (sira, (_, etiket)) in HEDEFLER.iter().enumerate() {
                                if filtre_cipi(ui, etiket, sira == hedef_sira).clicked()
                                    && sira != hedef_sira
                                {
                                    hedef_degis = Some(sira);
                                }
                            }
                        });
                    });
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal_top(|ui| {
                    // --- Sol: müşteri listesi -------------------------------
                    ui.allocate_ui_with_layout(
                        egui::vec2(290.0, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            bolum_etiketi(ui, "MÜŞTERİLER");
                            ui.add_space(4.0);
                            arama_kutusu(ui, &mut self.gelir_arama, "Müşteri ara");
                            ui.add_space(6.0);
                            let arama = arama_anahtari(&self.gelir_arama);
                            egui::ScrollArea::vertical()
                                .id_source("gelir_musteri_listesi")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    let mut bulundu = 0;
                                    for kayit in self.defter.kayitlar() {
                                        if !arama.is_empty()
                                            && !arama_anahtari(&kayit.ad).contains(&arama)
                                            && !arama_anahtari(&kayit.telefon).contains(&arama)
                                        {
                                            continue;
                                        }
                                        bulundu += 1;
                                        let aylik = self.gelir.aylik(&kayit.anahtar);
                                        let secili =
                                            self.gelir_secili.as_deref() == Some(&kayit.anahtar);
                                        let metin = if aylik > 0 {
                                            format!("{}  ·  {} TL", kayit.ad, fiyat_yaz(aylik))
                                        } else {
                                            kayit.ad.clone()
                                        };
                                        if liste_satiri(ui, secili, &metin).clicked() {
                                            secilen = Some(kayit.anahtar.clone());
                                        }
                                    }
                                    if bulundu == 0 {
                                        ui.label(
                                            egui::RichText::new(if self.defter.kayitlar().is_empty()
                                            {
                                                "Defter boş: önce Müşteri Bulma sekmesinde tarama yapın."
                                            } else {
                                                "Eşleşen müşteri yok."
                                            })
                                            .size(10.5)
                                            .color(DIM),
                                        );
                                    }
                                });
                        },
                    );

                    ui.separator();

                    // --- Sağ: paket ve hizmet seçimi ------------------------
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let Some((ad, telefon)) = musteri.clone() else {
                                ui.add_space(40.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        egui::RichText::new("Soldan bir müşteri seçin.")
                                            .size(12.0)
                                            .color(DIM),
                                    );
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Paket seçin ya da tek tek hizmet işaretleyin; aylık tutar ve teklif metni kendiliğinden oluşur.",
                                        )
                                        .size(10.0)
                                        .color(DIM),
                                    );
                                });
                                return;
                            };

                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(&ad)
                                            .font(baslik_font(16.0))
                                            .color(METIN),
                                    );
                                    ui.label(egui::RichText::new(&telefon).size(10.0).color(DIM));
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "aylık {} TL · tek seferlik {} TL",
                                                fiyat_yaz(gelir_kaydi.aylik()),
                                                fiyat_yaz(gelir_kaydi.tek_sefer_toplam())
                                            ))
                                            .size(11.0)
                                            .color(VURGU),
                                        );
                                    },
                                );
                            });
                            ui.add_space(6.0);

                            bolum_etiketi(ui, "PAKETLER");
                            ui.horizontal_wrapped(|ui| {
                                for paket in PAKETLER.iter() {
                                    let secili = gelir_kaydi.paket.as_deref() == Some(paket.kod);
                                    let metin = format!(
                                        "{} · {} TL",
                                        paket.ad,
                                        fiyat_yaz(paket.fiyat)
                                    );
                                    if filtre_cipi(ui, &metin, secili).clicked() && !secili {
                                        paket_sec = Some(paket.kod.to_string());
                                    }
                                }
                                if gelir_kaydi.paket.is_some()
                                    && hayalet_buton(ui, "Paketi bırak").clicked()
                                {
                                    paket_birak = true;
                                }
                            });
                            if let Some(paket) = gelir_kaydi.paket.as_deref().and_then(paket) {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}: {}",
                                        paket.ad,
                                        paket
                                            .hizmetler
                                            .iter()
                                            .filter_map(|kod| hizmet(kod))
                                            .map(|hizmet| hizmet.ad)
                                            .collect::<Vec<_>>()
                                            .join(" · ")
                                    ))
                                    .size(9.5)
                                    .color(SOLUK),
                                );
                            }
                            ui.add_space(8.0);

                            egui::ScrollArea::vertical()
                                .id_source("gelir_hizmetler")
                                .auto_shrink([false, false])
                                .max_height((ui.available_height() - 46.0).max(120.0))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    bolum_etiketi(ui, "AYLIK HİZMETLER");
                                    let mut grup = "";
                                    for hizmet in KATALOG.iter().filter(|hizmet| hizmet.aylik > 0) {
                                        if hizmet.grup != grup {
                                            grup = hizmet.grup;
                                            ui.add_space(6.0);
                                            ui.label(
                                                egui::RichText::new(grup).size(9.5).color(VURGU),
                                            );
                                        }
                                        let mut aktif = gelir_kaydi
                                            .hizmetler
                                            .iter()
                                            .any(|kod| kod == hizmet.kod);
                                        let etiket = format!(
                                            "{} · {} TL/ay{}",
                                            hizmet.ad,
                                            fiyat_yaz(hizmet.aylik),
                                            if hizmet.hazir { "  ✓" } else { "" }
                                        );
                                        if ui
                                            .checkbox(
                                                &mut aktif,
                                                egui::RichText::new(etiket).size(11.0),
                                            )
                                            .on_hover_text(hizmet.aciklama)
                                            .changed()
                                        {
                                            hizmet_degis =
                                                Some((hizmet.kod.to_string(), aktif));
                                        }
                                    }

                                    ui.add_space(12.0);
                                    bolum_etiketi(ui, "TEK SEFERLİK İŞLER");
                                    for hizmet in KATALOG.iter().filter(|h| h.tek_sefer > 0) {
                                        let mut aktif = gelir_kaydi
                                            .tek_seferler
                                            .iter()
                                            .any(|kod| kod == hizmet.kod);
                                        let etiket = format!(
                                            "{} · {} TL",
                                            hizmet.ad,
                                            fiyat_yaz(hizmet.tek_sefer)
                                        );
                                        if ui
                                            .checkbox(
                                                &mut aktif,
                                                egui::RichText::new(etiket).size(11.0),
                                            )
                                            .on_hover_text(hizmet.aciklama)
                                            .changed()
                                        {
                                            tek_sefer_degis =
                                                Some((hizmet.kod.to_string(), aktif));
                                        }
                                    }
                                });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if birincil_buton(ui, "Teklifi hazırla", true, None).clicked() {
                                    teklif_iste = true;
                                }
                                ui.add_space(6.0);
                                if ikincil_buton(ui, "Fatura kesildi", true, None).clicked() {
                                    fatura_iste = true;
                                }
                                ui.add_space(6.0);
                                if hayalet_buton(ui, "Metni kopyala").clicked() {
                                    kopyala_iste = true;
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if let Some(tarih) = &gelir_kaydi.son_fatura {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "son fatura · {tarih}"
                                                ))
                                                .size(9.5)
                                                .color(DIM),
                                            );
                                        }
                                    },
                                );
                            });
                        },
                    );
                });
            });

        // --- Değişiklikleri uygula ------------------------------------------
        if let Some(sira) = hedef_degis {
            self.gelir_hedef = sira;
        }
        if let Some(anahtar) = secilen {
            self.gelir_secili = Some(anahtar);
        }
        let Some(anahtar) = self.gelir_secili.clone() else {
            return;
        };
        let Some((ad, telefon)) = musteri else {
            return;
        };
        if let Some(paket_kodu) = paket_sec {
            self.gelir.paket_uygula(&anahtar, &ad, &paket_kodu);
            self.gelir.kaydet();
            let etiket = paket(&paket_kodu).map(|paket| paket.ad).unwrap_or("paket");
            self.logs
                .push(format!("[✓] {ad}: {etiket} paketi atandı."));
        }
        if paket_birak {
            self.gelir.paket_kaldir(&anahtar, &ad);
            self.gelir.kaydet();
        }
        if let Some((kod, ekle)) = hizmet_degis {
            self.gelir.hizmet_ayarla(&anahtar, &ad, &kod, ekle);
            self.gelir.kaydet();
        }
        if let Some((kod, ekle)) = tek_sefer_degis {
            self.gelir.tek_sefer_ayarla(&anahtar, &ad, &kod, ekle);
            self.gelir.kaydet();
        }
        if fatura_iste {
            self.gelir.fatura_kes(&anahtar, &ad);
            self.gelir.kaydet();
            self.logs
                .push(format!("[✓] {ad}: fatura kesildi olarak işaretlendi."));
        }
        if teklif_iste || kopyala_iste {
            let kayit = self.gelir.kayit(&anahtar).cloned().unwrap_or_default();
            let metin = teklif_metni(&ad, &kayit);
            if teklif_iste {
                match whatsapp_numarasi(&telefon) {
                    Some(numara) => {
                        if sohbet_ac(&numara, &metin, &mut self.logs) {
                            self.logs
                                .push(format!("[✓] {ad}: teklif WhatsApp'ta açıldı."));
                        }
                    }
                    None => {
                        ctx.output_mut(|cikti| cikti.copied_text = metin);
                        self.logs.push(format!(
                            "[i] {ad}: telefon yok, teklif panoya kopyalandı."
                        ));
                    }
                }
            } else {
                ctx.output_mut(|cikti| cikti.copied_text = metin);
                self.logs
                    .push("[✓] Teklif metni panoya kopyalandı.".to_string());
            }
        }
    }
}
