//! Toplu işlem tablosu: müşterileri işaretleyip tek tuşla satış kurulumu yapar.
//!
//! Tabloda seçilen her müşteriye sırayla uygulanan adımlar:
//!
//! 1. Seçili paket yazılır (`gelir.paket_uygula`).
//! 2. Teklif metni hazırlanır (`teklif_metni` + `teklif_ekle`).
//! 3. Paketin teslim dosyaları üretilir (`teslimat::yaz`).
//! 4. Fatura kesildi olarak işaretlenir.
//!
//! Hepsi yerelde ve ücretsizdir; dış servis çağrılmaz.

use crate::*;
use eframe::egui;
use std::collections::HashSet;

/// Seçili müşterilerin aylık toplamı.
pub fn toplam_aylik(gelir: &gelir::Gelir, secim: &HashSet<String>) -> u32 {
    gelir
        .kayitlar()
        .iter()
        .filter(|kayit| secim.contains(&kayit.anahtar))
        .map(|kayit| kayit.aylik())
        .sum()
}

/// Seçili müşterilerin tek seferlik iş toplamı.
pub fn toplam_tek_sefer(gelir: &gelir::Gelir, secim: &HashSet<String>) -> u32 {
    gelir
        .kayitlar()
        .iter()
        .filter(|kayit| secim.contains(&kayit.anahtar))
        .map(|kayit| kayit.tek_sefer_toplam())
        .sum()
}

/// Aylık hedefe oran (%).
pub fn hedef_orani(tutar: u32) -> u32 {
    ((tutar as u64 * 100) / otomasyon::HEDEF_TL as u64).min(100) as u32
}

/// Sabit genişlikte, soldan hizalı tablo hücresi.
fn hucre(ui: &mut egui::Ui, genislik: f32, icerik: egui::RichText) {
    ui.allocate_ui_with_layout(
        egui::vec2(genislik, 16.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(genislik);
            ui.add(egui::Label::new(icerik).truncate(true));
        },
    );
}

impl LeadApp {
    /// Gelir katmanının "Tablo" sayfası: seç → uygula.
    pub(crate) fn tablo_ekrani(&mut self, ui: &mut egui::Ui) {
        let paket_sira = self.tablo_paket.min(gelir::PAKETLER.len() - 1);
        let paket = &gelir::PAKETLER[paket_sira];
        let gun = bugun_anahtari();

        let secili_aylik = toplam_aylik(&self.gelir, &self.tablo_secim);
        let secili_tek = toplam_tek_sefer(&self.gelir, &self.tablo_secim);

        let mut uygula = false;
        let mut teklifle = false;
        let mut uret = false;
        let mut fatura = false;
        let mut temizle = false;
        let mut hepsini_sec = false;
        let mut secim_degis: Option<(String, bool)> = None;
        let mut paket_degis: Option<usize> = None;

        // Tablo satırları: çizim sırasında ödünç çakışmasın diye kopyalanır.
        let satirlar: Vec<(String, String, String, u8, String, u32, bool)> = self
            .defter
            .kayitlar()
            .iter()
            .map(|musteri| {
                let gelir_kaydi = self.gelir.kayit(&musteri.anahtar);
                let paket_adi = gelir_kaydi
                    .and_then(|kayit| kayit.paket.as_deref())
                    .and_then(gelir::paket)
                    .map(|paket| paket.ad.to_string())
                    .unwrap_or_else(|| "—".to_string());
                (
                    musteri.anahtar.clone(),
                    musteri.ad.clone(),
                    musteri.telefon.clone(),
                    musteri.puan,
                    paket_adi,
                    gelir_kaydi.map(|kayit| kayit.aylik()).unwrap_or(0),
                    self.gelir.bugun_teklif_var(&musteri.anahtar, &gun),
                )
            })
            .collect();

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("TOPLU İŞLEM")
                    .font(baslik_font(14.0))
                    .color(METIN),
            );
            ui.label(
                egui::RichText::new(format!(
                    "seçili {} · aylık {} TL · tek seferlik {} TL · hedefe %{}",
                    self.tablo_secim.len(),
                    otomasyon::tl(secili_aylik),
                    otomasyon::tl(secili_tek),
                    hedef_orani(secili_aylik)
                ))
                .size(10.5)
                .color(SOLUK),
            );
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("PAKET").size(9.0).color(DIM));
            for (sira, aday) in gelir::PAKETLER.iter().enumerate() {
                let metin = format!("{} · {} TL", aday.ad, otomasyon::tl(aday.fiyat));
                if filtre_cipi(ui, &metin, sira == paket_sira).clicked() && sira != paket_sira {
                    paket_degis = Some(sira);
                }
            }
        });
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            if birincil_buton(ui, "Seçilenlere uygula", true, None).clicked() {
                uygula = true;
            }
            ui.add_space(6.0);
            if ikincil_buton(ui, "Teklif hazırla", true, None).clicked() {
                teklifle = true;
            }
            ui.add_space(6.0);
            if ikincil_buton(ui, "Dosya üret", true, None).clicked() {
                uret = true;
            }
            ui.add_space(6.0);
            if ikincil_buton(ui, "Fatura kes", true, None).clicked() {
                fatura = true;
            }
            ui.add_space(6.0);
            if hayalet_buton(ui, "Tümünü seç").clicked() {
                hepsini_sec = true;
            }
            if hayalet_buton(ui, "Temizle").clicked() {
                temizle = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{}: {}", paket.ad, paket.hizmetler.join(" · ")))
                        .size(9.0)
                        .color(DIM),
                );
            });
        });
        ui.add_space(8.0);

        // --- Tablo başlığı ---------------------------------------------------
        let sutunlar: [(&str, f32); 6] = [
            ("İŞLETME", 230.0),
            ("TELEFON", 130.0),
            ("PUAN", 46.0),
            ("PAKET", 96.0),
            ("AYLIK", 86.0),
            ("TEKLİF", 52.0),
        ];
        ui.horizontal(|ui| {
            ui.allocate_exact_size(egui::vec2(26.0, 16.0), egui::Sense::hover());
            for (baslik, genislik) in sutunlar {
                hucre(
                    ui,
                    genislik,
                    egui::RichText::new(baslik).size(8.5).color(DIM),
                );
            }
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .id_source("tablo_satirlar")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for (anahtar, ad, telefon, puan, paket_adi, aylik, teklif_var) in &satirlar {
                    let secili = self.tablo_secim.contains(anahtar);
                    let arka = if secili { GIRINTI } else { PANEL };
                    egui::Frame::none()
                        .fill(arka)
                        .inner_margin(egui::Margin::symmetric(2.0, 3.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(26.0, 16.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        let mut isaret = secili;
                                        if ui.checkbox(&mut isaret, "").changed() {
                                            secim_degis = Some((anahtar.clone(), isaret));
                                        }
                                    },
                                );
                                let renk = if aylik > &0 { YESIL } else { METIN };
                                hucre(ui, 230.0, egui::RichText::new(ad).size(11.0).color(renk));
                                hucre(
                                    ui,
                                    130.0,
                                    egui::RichText::new(telefon).size(10.0).color(SOLUK),
                                );
                                hucre(
                                    ui,
                                    46.0,
                                    egui::RichText::new(puan.to_string())
                                        .size(10.0)
                                        .color(SOLUK),
                                );
                                hucre(
                                    ui,
                                    96.0,
                                    egui::RichText::new(paket_adi).size(10.0).color(SOLUK),
                                );
                                hucre(
                                    ui,
                                    86.0,
                                    egui::RichText::new(if *aylik > 0 {
                                        format!("{} TL", otomasyon::tl(*aylik))
                                    } else {
                                        "—".to_string()
                                    })
                                    .size(10.0)
                                    .color(renk),
                                );
                                hucre(
                                    ui,
                                    52.0,
                                    egui::RichText::new(if *teklif_var { "var" } else { "—" })
                                        .size(10.0)
                                        .color(if *teklif_var { YESIL } else { DIM }),
                                );
                            });
                        });
                }
            });

        // --- Değişiklikleri uygula ------------------------------------------
        if let Some((anahtar, secili)) = secim_degis {
            if secili {
                self.tablo_secim.insert(anahtar);
            } else {
                self.tablo_secim.remove(&anahtar);
            }
        }
        if let Some(sira) = paket_degis {
            self.tablo_paket = sira;
        }
        if hepsini_sec {
            self.tablo_secim = satirlar.iter().map(|satir| satir.0.clone()).collect();
        }
        if temizle {
            self.tablo_secim.clear();
        }
        if uygula || teklifle || uret || fatura {
            let anahtarlar: Vec<String> = self.tablo_secim.iter().cloned().collect();
            let mut dosya = 0;
            let mut toplam = 0;
            for anahtar in &anahtarlar {
                let Some(musteri) = self.defter.kayit(anahtar).cloned() else {
                    continue;
                };
                if uygula {
                    self.gelir.paket_uygula(anahtar, &musteri.ad, paket.kod);
                }
                if uygula || teklifle {
                    if let Some(kayit) = self.gelir.kayit(anahtar).cloned() {
                        let teklif = gelir::Teklif {
                            anahtar: anahtar.clone(),
                            ad: musteri.ad.clone(),
                            gun: gun.clone(),
                            paket: kayit.paket.clone(),
                            aylik: kayit.aylik(),
                            tek_sefer: kayit.tek_sefer_toplam(),
                            ozet: format!("{} · {} TL/ay", paket.ad, otomasyon::tl(kayit.aylik())),
                            metin: gelir::teklif_metni(&musteri.ad, &kayit),
                            gonderildi: false,
                        };
                        self.gelir.teklif_ekle(teklif);
                    }
                }
                if uygula || uret {
                    if let Some(kayit) = self.gelir.kayit(anahtar).cloned() {
                        if kayit.aylik() > 0 && teslimat::yaz(&musteri, &kayit, &gun).is_ok() {
                            dosya += 1;
                        }
                    }
                }
                if uygula || fatura {
                    self.gelir.fatura_kes(anahtar, &musteri.ad);
                }
                toplam += self.gelir.aylik(anahtar);
            }
            self.gelir.kaydet();
            self.logs.push(format!(
                "[✓] {} müşteri işlendi · seçili aylık {} TL · {} dosya üretildi.",
                anahtarlar.len(),
                otomasyon::tl(toplam),
                dosya
            ));
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gelir_ornek() -> gelir::Gelir {
        gelir::Gelir::metinden(
            "{\"kayitlar\":[\
             {\"anahtar\":\"a\",\"ad\":\"A\",\"paket\":\"buyume\",\"paket_fiyati\":5000},\
             {\"anahtar\":\"b\",\"ad\":\"B\",\"hizmetler\":[],\"tek_seferler\":[\"seo_audit\"]},\
             {\"anahtar\":\"c\",\"ad\":\"C\"}]}",
        )
    }

    #[test]
    fn secili_aylik_toplam_hesaplanir() {
        let gelir = gelir_ornek();
        let mut secim = HashSet::new();
        secim.insert("a".to_string());
        secim.insert("b".to_string());
        assert_eq!(toplam_aylik(&gelir, &secim), 5_000);
        assert_eq!(toplam_tek_sefer(&gelir, &secim), 2_500);
    }

    #[test]
    fn hedef_orani_yuzde_verir() {
        assert_eq!(hedef_orani(0), 0);
        assert_eq!(hedef_orani(5_000), 2);
        assert_eq!(hedef_orani(200_000), 100);
        assert_eq!(hedef_orani(400_000), 100);
    }
}
