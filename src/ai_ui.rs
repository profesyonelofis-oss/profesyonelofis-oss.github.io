//! Yerel modelin arayüz tarafı: durum yoklaması, taslak istekleri ve panel.
//!
//! Model yoksa panel "yerel model yok" der ve düğmeler kapalı kalır; gelen
//! kutusunun geri kalanı (anahtar sözcük niyeti, elle yanıt) aynen çalışır.

use crate::*;
use eframe::egui;

impl LeadApp {
    /// Model durumunu 60 saniyede bir yoklar ve yeni gelen mesajların
    /// niyetini otomatik doğrular (tek seferde tek iş).
    pub(crate) fn ai_tazele(&mut self, _ctx: &egui::Context) {
        let gerekli = self
            .ai_son
            .map(|zaman| zaman.elapsed().as_secs() >= 60)
            .unwrap_or(true);
        if gerekli {
            self.ai_durum = ai::yokla();
            self.ai_son = Some(Instant::now());
        }
        if !self.ai_durum.aktif || self.ai_rx.is_some() {
            return;
        }
        // Henüz sınıflanmamış gelen mesajlar: her biri bir kez denenir.
        let aday = self
            .gelen
            .kayitlar()
            .iter()
            .find(|kayit| kayit.yon == gelen::Yon::Gelen && kayit.niyet == gelen::Niyet::Bilinmiyor)
            .map(|kayit| {
                (
                    format!("{}|{}|{}", kayit.zaman, kayit.telefon, kayit.metin),
                    kayit.telefon.clone(),
                    kayit.metin.clone(),
                )
            });
        if let Some((anahtar, telefon, metin)) = aday {
            if self.ai_denenen.insert(anahtar) {
                self.ai_niyet_iste(&telefon, &metin);
            }
        }
    }

    /// Modelin cevabını toplar ve gelen kutusuna uygular.
    pub(crate) fn ai_sonucu_al(&mut self) {
        let Some(rx) = &self.ai_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(sonuc) => {
                self.ai_rx = None;
                if let Some(niyet) = sonuc.niyet {
                    let satirlar: Vec<usize> = self
                        .gelen
                        .kayitlar()
                        .iter()
                        .enumerate()
                        .filter(|(_, kayit)| {
                            kayit.yon == gelen::Yon::Gelen
                                && kayit.telefon == sonuc.telefon
                                && kayit.niyet == gelen::Niyet::Bilinmiyor
                        })
                        .map(|(sira, _)| sira)
                        .collect();
                    if satirlar.is_empty() {
                        self.logs.push(format!(
                            "[i] AI niyeti: {} · {}/{}",
                            niyet.etiket(),
                            niyet.sira(),
                            gelen::Niyet::HEPSI.len()
                        ));
                    } else {
                        self.gelen.niyet_ayarla(&satirlar, niyet);
                        self.gelen.kaydet();
                        self.logs.push(format!(
                            "[✓] AI niyeti doğruladı: {} · {}",
                            niyet.etiket(),
                            sonuc.telefon
                        ));
                    }
                }
                if let Some(yanit) = sonuc.yanit {
                    if self.gelen_yanit.trim().is_empty() {
                        self.gelen_yanit = yanit.clone();
                    }
                    self.ai_taslak = Some(yanit);
                    self.logs
                        .push("[i] AI yanıt taslağı hazır; göndermeden önce okuyun.".to_string());
                }
                if let Some(hata) = sonuc.hata {
                    self.logs.push(format!("[!] Yerel model: {hata}"));
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.ai_rx = None,
        }
    }

    fn ai_niyet_iste(&mut self, telefon: &str, metin: &str) {
        if !self.ai_durum.aktif {
            return;
        }
        let istek = ai::Istek::Niyet {
            telefon: telefon.to_string(),
            metin: metin.to_string(),
        };
        self.ai_rx = Some(ai::baslat(self.ai_durum.model.clone(), istek));
    }

    /// Seçili yazışma için taslak ister.
    fn ai_taslak_iste(&mut self) {
        if !self.ai_durum.aktif {
            return;
        }
        let Some(sira) = self.gelen_secili else {
            return;
        };
        let kanallar = self.gelen.kanallar();
        let Some(kanal) = kanallar.get(sira) else {
            return;
        };
        // Müşterinin son mesajı isteme girer.
        let son = kanal
            .satirlar
            .iter()
            .rev()
            .find_map(|indeks| {
                self.gelen
                    .kayitlar()
                    .get(*indeks)
                    .filter(|kayit| kayit.yon == gelen::Yon::Gelen)
            })
            .map(|kayit| kayit.metin.clone())
            .unwrap_or_else(|| "Merhaba".to_string());
        let sektor = self
            .leads
            .iter()
            .find(|lead| lead.phone == kanal.telefon)
            .map(|lead| lead.category.clone())
            .unwrap_or_else(|| "yerel işletme".to_string());
        let isletme = if kanal.ad.trim().is_empty() {
            "işletme".to_string()
        } else {
            kanal.ad.clone()
        };
        let istek = ai::Istek::Yanit {
            telefon: kanal.telefon.clone(),
            isletme,
            sektor,
            metin: son,
            niyet: kanal.niyet,
        };
        self.ai_rx = Some(ai::baslat(self.ai_durum.model.clone(), istek));
        self.logs.push(format!(
            "[i] Yerel model yanıt taslağı yazıyor ({}).",
            self.ai_durum.model
        ));
    }

    /// Katman 4'ün sağ panelindeki "Yerel model" bölümü.
    pub(crate) fn ai_paneli(&mut self, ui: &mut egui::Ui) {
        let mut yokla = false;
        let mut taslak = false;

        bolum_etiketi(ui, "YEREL MODEL");
        ui.horizontal(|ui| {
            pill(
                ui,
                if self.ai_durum.aktif {
                    "aktif"
                } else {
                    "kapalı"
                },
                if self.ai_durum.aktif { YESIL } else { DIM },
            );
            if self.ai_durum.aktif {
                ui.label(
                    egui::RichText::new(&self.ai_durum.model)
                        .font(kod_font(9.0))
                        .color(SOLUK),
                );
            }
            if self.ai_rx.is_some() {
                ui.add(egui::Spinner::new().size(10.0));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if hayalet_buton(ui, "Yokla").clicked() {
                    yokla = true;
                }
            });
        });
        ui.label(
            egui::RichText::new(format!(
                "{} · {}",
                if self.ai_durum.mesaj.is_empty() {
                    "durum bilinmiyor"
                } else {
                    &self.ai_durum.mesaj
                },
                self.ai_durum.url
            ))
            .size(8.5)
            .color(DIM),
        );
        ui.add_space(5.0);
        if ikincil_buton(
            ui,
            "AI ile yanıt taslağı yaz",
            self.ai_durum.aktif && self.gelen_secili.is_some() && self.ai_rx.is_none(),
            Some(egui::vec2(ui.available_width(), 28.0)),
        )
        .clicked()
        {
            taslak = true;
        }
        if let Some(metin) = self.ai_taslak.clone() {
            ui.add_space(5.0);
            egui::Frame::none()
                .fill(GIRINTI)
                .stroke(egui::Stroke::new(1.0_f32, KENAR))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("AI TASLAĞI")
                                .font(baslik_font(8.0))
                                .color(DIM),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if hayalet_buton(ui, "Yanıta aktar").clicked() {
                                self.gelen_yanit = metin.clone();
                            }
                        });
                    });
                    ui.label(
                        egui::RichText::new(kisalt_metin(ui, &metin, ui.available_width(), 9.5))
                            .size(9.5)
                            .color(SOLUK),
                    );
                });
        }
        ui.label(
            egui::RichText::new("Taslak öneridir; gönderme kararı sizde.")
                .size(8.0)
                .color(DIM),
        );
        ui.add_space(12.0);

        if yokla {
            self.ai_durum = ai::yokla();
            self.ai_son = Some(Instant::now());
            self.logs
                .push(format!("[i] Yerel model: {}", self.ai_durum.mesaj));
        }
        if taslak {
            self.ai_taslak_iste();
        }
    }
}
