/// Yerel video içe aktarma işinin arka plan olayları.
enum StokOlay {
    Hazir(stok_video::Hazir),
    Hata(String),
    Bitti,
}

impl LeadApp {
    /// "Sinematik varlıklar": yerel video içe aktarma. Stok API'leri kapalı
    /// olduğu için panelde anahtar alanı yok; ağa hiç çıkılmaz.
    fn stok_paneli(&mut self, ui: &mut egui::Ui) {
        let mut olaylar = Vec::new();
        if let Some(kanal) = &self.stok_islem {
            while let Ok(olay) = kanal.try_recv() {
                olaylar.push(olay);
            }
        }
        for olay in olaylar {
            match olay {
                StokOlay::Hazir(hazir) => {
                    self.stok_durum = format!(
                        "Kaydedildi: media/sektor/{}/{} · {:.1} MB video · {:.0} KB poster · {:.0} sn kullanıldı.",
                        hazir.grup,
                        hazir.dosya_adi,
                        hazir.video_bayt as f64 / 1_048_576.0,
                        hazir.poster_bayt as f64 / 1024.0,
                        hazir.sure
                    );
                    self.stok_kutuphane = stok_video::kutuphane();
                }
                StokOlay::Hata(hata) => self.stok_durum = hata,
                StokOlay::Bitti => {
                    self.stok_islem = None;
                    self.stok_mesgul = false;
                }
            }
        }

        bolum_etiketi(ui, "SİNEMATİK VARLIKLAR");
        ui.label(
            egui::RichText::new(
                "Yerel video · mp4, mov, webm · 720p · her kare anahtar kare · sessiz · en fazla 8 sn",
            )
            .size(9.8)
            .color(DIM),
        );
        if !stok_video::ffmpeg_var() {
            ui.label(
                egui::RichText::new(
                    "ffmpeg bulunamadı: video işlenemez. https://ffmpeg.org/download.html",
                )
                .size(10.0)
                .color(TURUNCU),
            );
        }
        ui.add_space(4.0);

        ui.horizontal_wrapped(|ui| {
            for grup in stok_video::GRUPLAR {
                let secili = self.stok_grup == grup.id;
                let metin = egui::RichText::new(grup.ad)
                    .size(10.0)
                    .color(if secili { KOYU_YAZI } else { METIN });
                let buton = egui::Button::new(metin)
                    .fill(if secili { VURGU } else { KART })
                    .rounding(egui::Rounding::same(999.0))
                    .min_size(egui::vec2(0.0, 22.0));
                if ui.add(buton).clicked() && !secili {
                    self.stok_grup = grup.id.to_string();
                }
            }
        });
        ui.add_space(4.0);

        // Sürükle-bırak: pencereye bırakılan dosya yol alanına düşer.
        let birakilan: Option<std::path::PathBuf> =
            ui.ctx()
                .input(|girdi| girdi.raw.dropped_files.iter().find_map(|dosya| dosya.path.clone()));
        if let Some(yol) = birakilan {
            self.stok_yol = yol.to_string_lossy().to_string();
            self.stok_durum = format!(
                "Dosya seçildi: {}",
                yol.file_name()
                    .map(|ad| ad.to_string_lossy().to_string())
                    .unwrap_or_default()
            );
        }
        let ustunde = ui.ctx().input(|girdi| !girdi.raw.hovered_files.is_empty());
        let kenar = if ustunde {
            VURGU
        } else {
            egui::Color32::from_rgb(52, 52, 58)
        };
        egui::Frame::none()
            .fill(KART)
            .stroke(egui::Stroke::new(1.0, kenar))
            .rounding(10.0)
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    egui::RichText::new(
                        "Videoyu bu pencereye sürükleyin ya da tam dosya yolunu yapıştırın",
                    )
                    .size(10.5)
                    .color(DIM),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.stok_yol)
                        .hint_text("C:\\video\\kuafor.mp4")
                        .desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Başlangıç sn").size(10.0).color(DIM));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.stok_baslangic).desired_width(56.0),
                    );
                    ui.label(
                        egui::RichText::new("8 sn'den uzun videoda en iyi bölümü seçer")
                            .size(9.5)
                            .color(DIM),
                    );
                });
                ui.add(
                    egui::TextEdit::singleline(&mut self.stok_linki)
                        .hint_text("Kaynak bağlantısı (isteğe bağlı)")
                        .desired_width(f32::INFINITY),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.stok_cekim)
                        .hint_text("Çekimi yapan (isteğe bağlı)")
                        .desired_width(f32::INFINITY),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.stok_lisans)
                        .hint_text("Lisans notu (isteğe bağlı)")
                        .desired_width(f32::INFINITY),
                );
                ui.checkbox(&mut self.stok_atif_goster, "Altbilgide atıf göster");
                ui.add_space(4.0);
                if ui
                    .add_enabled(
                        !self.stok_mesgul,
                        egui::Button::new("Dönüştür ve kaydet"),
                    )
                    .clicked()
                {
                    let ctx = ui.ctx().clone();
                    self.stok_baslat(ctx);
                }
            });

        if !self.stok_durum.is_empty() {
            ui.label(
                egui::RichText::new(&self.stok_durum)
                    .size(10.5)
                    .color(if self.stok_mesgul { DIM } else { YESIL }),
            );
        }

        let kutuphane = self.stok_kutuphane.clone();
        if kutuphane.is_empty() {
            ui.label(
                egui::RichText::new(
                    "Kayıtlı video yok; üretimde shader'lı sinematik sürüm kullanılır.",
                )
                .size(10.0)
                .color(DIM),
            );
        } else {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("KAYITLI VİDEOLAR").size(9.5).color(DIM));
            ui.label(
                egui::RichText::new(
                    "Aynı grupta birden fazla video varsa işletme adından türeyen tohumla biri seçilir.",
                )
                .size(9.5)
                .color(DIM),
            );
            let mut silinecek: Option<(String, String)> = None;
            for bilgi in &kutuphane {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{} · {}", bilgi.grup, bilgi.dosya_adi))
                            .size(10.0)
                            .color(YESIL),
                    )
                    .on_hover_text(bilgi.kunye());
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.1} MB · {:.0} KB",
                            bilgi.video_bayt as f64 / 1_048_576.0,
                            bilgi.poster_bayt as f64 / 1024.0
                        ))
                        .size(9.5)
                        .color(DIM),
                    );
                    if ui.small_button("Sil").clicked() {
                        silinecek = Some((bilgi.grup.clone(), bilgi.dosya_adi.clone()));
                    }
                });
            }
            if let Some((grup, dosya)) = silinecek {
                match stok_video::sil_kayit(&grup, &dosya) {
                    Ok(()) => {
                        self.stok_kutuphane = stok_video::kutuphane();
                        self.stok_durum = format!("{dosya} silindi.");
                    }
                    Err(hata) => self.stok_durum = hata,
                }
            }
        }
    }

    fn stok_baslat(&mut self, ctx: egui::Context) {
        if self.stok_mesgul {
            return;
        }
        let istek = stok_video::YerelIstek {
            kaynak_yol: self.stok_yol.trim().to_string(),
            baslangic: self
                .stok_baslangic
                .trim()
                .replace(',', ".")
                .parse::<f64>()
                .unwrap_or(0.0),
            kaynak_linki: self.stok_linki.trim().to_string(),
            cekim: self.stok_cekim.trim().to_string(),
            lisans: self.stok_lisans.trim().to_string(),
            atif_gerekli: self.stok_atif_goster,
        };
        let grup = self.stok_grup.clone();
        let (gonderen, kanal) = channel();
        self.stok_islem = Some(kanal);
        self.stok_mesgul = true;
        self.stok_durum =
            "Video dönüştürülüyor: 720p, her kare anahtar kare, sessiz…".to_string();
        thread::spawn(move || {
            let sonuc = stok_video::yerel_hazirla(&grup, &istek);
            let _ = gonderen.send(match sonuc {
                Ok(hazir) => StokOlay::Hazir(hazir),
                Err(hata) => StokOlay::Hata(hata),
            });
            let _ = gonderen.send(StokOlay::Bitti);
            ctx.request_repaint();
        });
    }
}
