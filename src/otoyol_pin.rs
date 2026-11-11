//! O-5 (İzmir–İstanbul) otoyolu pin haritası.
//!
//! Koridordaki bölgeler haritada numaralı pinlerle, aralarında otoyol hattı
//! olacak şekilde çizilir. Seçilen pin için anahtar kelimeler, pin bağlantısı
//! ve pinleme adımları gösterilir.
//!
//! Pinler **bölge işaretleridir**: her biri "bu bölgede hizmet veriyoruz"
//! noktasıdır, Google'a eklenen işletme kaydı değildir.

use crate::*;
use eframe::egui;

/// Koridordaki tek bir bölge pini.
pub struct Nokta {
    pub il: &'static str,
    pub ilce: &'static str,
    pub enlem: f64,
    pub boylam: f64,
}

/// O-5 hattı: İstanbul çıkışlarından İzmir'e kadar sıralı bölgeler.
pub const NOKTALAR: &[Nokta] = &[
    Nokta {
        il: "İstanbul",
        ilce: "Tuzla",
        enlem: 40.8175,
        boylam: 29.3000,
    },
    Nokta {
        il: "İstanbul",
        ilce: "Pendik",
        enlem: 40.8770,
        boylam: 29.2300,
    },
    Nokta {
        il: "İstanbul",
        ilce: "Kartal",
        enlem: 40.9000,
        boylam: 29.1900,
    },
    Nokta {
        il: "Kocaeli",
        ilce: "Gebze",
        enlem: 40.8000,
        boylam: 29.4300,
    },
    Nokta {
        il: "Kocaeli",
        ilce: "Dilovası",
        enlem: 40.7800,
        boylam: 29.5350,
    },
    Nokta {
        il: "Kocaeli",
        ilce: "Çayırova",
        enlem: 40.8200,
        boylam: 29.3750,
    },
    Nokta {
        il: "Yalova",
        ilce: "Altınova",
        enlem: 40.7000,
        boylam: 29.5100,
    },
    Nokta {
        il: "Yalova",
        ilce: "Çiftlikköy",
        enlem: 40.6650,
        boylam: 29.3200,
    },
    Nokta {
        il: "Yalova",
        ilce: "Çınarcık",
        enlem: 40.6450,
        boylam: 29.1200,
    },
    Nokta {
        il: "Bursa",
        ilce: "Orhangazi",
        enlem: 40.4900,
        boylam: 29.3100,
    },
    Nokta {
        il: "Bursa",
        ilce: "Gemlik",
        enlem: 40.4300,
        boylam: 29.1500,
    },
    Nokta {
        il: "Bursa",
        ilce: "Nilüfer",
        enlem: 40.2150,
        boylam: 28.9700,
    },
    Nokta {
        il: "Bursa",
        ilce: "Karacabey",
        enlem: 40.2150,
        boylam: 28.3600,
    },
    Nokta {
        il: "Balıkesir",
        ilce: "Merkez",
        enlem: 39.6500,
        boylam: 27.8900,
    },
    Nokta {
        il: "Balıkesir",
        ilce: "Savaştepe",
        enlem: 39.3836,
        boylam: 27.6553,
    },
    Nokta {
        il: "Balıkesir",
        ilce: "Susurluk",
        enlem: 39.9100,
        boylam: 28.1600,
    },
    Nokta {
        il: "Manisa",
        ilce: "Soma",
        enlem: 39.1900,
        boylam: 27.6100,
    },
    Nokta {
        il: "Manisa",
        ilce: "Kırkağaç",
        enlem: 39.1000,
        boylam: 27.6700,
    },
    Nokta {
        il: "Manisa",
        ilce: "Akhisar",
        enlem: 38.9200,
        boylam: 27.8400,
    },
    Nokta {
        il: "Manisa",
        ilce: "Saruhanlı",
        enlem: 38.7350,
        boylam: 27.5650,
    },
    Nokta {
        il: "Manisa",
        ilce: "Şehzadeler",
        enlem: 38.6100,
        boylam: 27.4300,
    },
    Nokta {
        il: "Manisa",
        ilce: "Yunusemre",
        enlem: 38.6000,
        boylam: 27.4000,
    },
    Nokta {
        il: "İzmir",
        ilce: "Aliağa",
        enlem: 38.8000,
        boylam: 26.9700,
    },
    Nokta {
        il: "İzmir",
        ilce: "Menemen",
        enlem: 38.6100,
        boylam: 27.0700,
    },
    Nokta {
        il: "İzmir",
        ilce: "Çiğli",
        enlem: 38.5000,
        boylam: 27.0600,
    },
    Nokta {
        il: "İzmir",
        ilce: "Bornova",
        enlem: 38.4700,
        boylam: 27.2200,
    },
    Nokta {
        il: "İzmir",
        ilce: "Gaziemir",
        enlem: 38.3900,
        boylam: 27.1300,
    },
    Nokta {
        il: "İzmir",
        ilce: "Torbalı",
        enlem: 38.1600,
        boylam: 27.3600,
    },
    Nokta {
        il: "Manisa",
        ilce: "Turgutlu",
        enlem: 38.4953,
        boylam: 27.7061,
    },
    Nokta {
        il: "İzmir",
        ilce: "Kemalpaşa",
        enlem: 38.4286,
        boylam: 27.4153,
    },
    Nokta {
        il: "Otoyol",
        ilce: "O-5 Otoyolu",
        enlem: 39.4500,
        boylam: 28.4500,
    },
    Nokta {
        il: "Otoyol",
        ilce: "İzmir–İstanbul Otoyolu",
        enlem: 39.9000,
        boylam: 28.3000,
    },
];

/// Pin anahtarı: il + ilçe.
pub fn anahtar(nokta: &Nokta) -> String {
    format!("{}|{}", nokta.il, nokta.ilce)
}

/// Pinin etiketi (haritada ve listede).
pub fn etiket(nokta: &Nokta) -> String {
    if nokta.il == "Otoyol" {
        nokta.ilce.to_string()
    } else {
        format!("{} · {}", nokta.ilce, nokta.il)
    }
}

/// Her pinin taşıdığı hizmet kelimeleri; ilçe/otoyol adıyla birleştirilir.
pub const HIZMETLER: &[&str] = &[
    "çekici",
    "oto kurtarma",
    "yol yardım 7/24",
    "en yakın çekici",
    "benzin destek",
    "mazot takviye",
    "akü takviye",
    "akücü",
];

/// Bu pine bağlanacak anahtar kelimeler: **bölge/ilçe adı eklenmez**.
///
/// Arama "çekici", "akü takviye" gibi bölge yazılmadan yapılır; eşleşme
/// pinin gerçek konumundan (yakınlık) gelir. İlçe adı metinlere zaten sayfa
/// başlığı, adres ve pin bağlantısıyla giriyor.
pub fn anahtar_kelimeler(_sektor: &str, _nokta: &Nokta) -> Vec<String> {
    HIZMETLER
        .iter()
        .map(|hizmet| (*hizmet).to_string())
        .collect()
}

/// Pin için Google Maps bağlantısı (ana anahtar kelime ile).
pub fn pin_link(sektor: &str, nokta: &Nokta) -> String {
    bolgesel::bolge_link(sektor, nokta.il, nokta.ilce)
}

/// Pinlemenin nasıl yapıldığını anlatan adımlar.
pub fn adimlar() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "1 · Haritada pini gör",
            "Bölge, O-5 hattı üzerinde numaralı bir pin olarak işaretli. Tıklayınca o bölgenin kartı açılır.",
        ),
        (
            "2 · Anahtar kelime seç",
            "Her pin aynı 8 anahtar kelimeyi hedefler: çekici, oto kurtarma, yol yardım 7/24, en yakın çekici, benzin destek, mazot takviye, akü takviye, akücü. İlçe adı eklenmez; arama yakınlıkla eşleşir.",
        ),
        (
            "3 · Pin bağlantısı",
            "Seçilen pinden Google Maps bağlantısı üretilir; profilde site ve mesajlarda bu bağlantı kullanılır.",
        ),
        (
            "4 · Hizmet alanına ekle",
            "Profilde hizmet alanına bölgeyi ekle, sayfayı yayına al. Pin kopya işletme kaydı değildir.",
        ),
    ]
}

impl LeadApp {
    /// O-5 pin haritası ekranı.
    pub(crate) fn otoyol_pin_ekrani(&mut self, ctx: &egui::Context) {
        let sektor = self
            .pin_secili
            .as_ref()
            .and_then(|anahtar| self.pinler.kayitlar.get(anahtar))
            .map(|pin| bolgesel::sektor_tahmin(&pin.ad, &pin.adres))
            .unwrap_or_else(|| "Oto kurtarma".to_string());
        if self.otoyol_secili.is_none() {
            self.otoyol_secili = NOKTALAR
                .iter()
                .find(|nokta| nokta.ilce == "Saruhanlı")
                .map(anahtar);
        }
        let secili = self
            .otoyol_secili
            .clone()
            .and_then(|secim| NOKTALAR.iter().find(|nokta| anahtar(nokta) == secim));
        let mut yeni_secim: Option<String> = None;
        let mut sigdir = false;
        let mut yakinlas = false;
        let mut uzaklas = false;
        let mut kopyala = false;
        let mut ac = false;

        // Harita ve liste kuzeyden güneye sıralı: hat İstanbul'dan İzmir'e düz iner.
        // Otoyol etiket pinleri hattın sonuna eklenir, çizgiye girmez.
        let mut sirali: Vec<&Nokta> = NOKTALAR
            .iter()
            .filter(|nokta| nokta.il != "Otoyol")
            .collect();
        sirali.sort_by(|a, b| {
            b.enlem
                .partial_cmp(&a.enlem)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    b.boylam
                        .partial_cmp(&a.boylam)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        sirali.extend(NOKTALAR.iter().filter(|nokta| nokta.il == "Otoyol"));

        let isaretler: Vec<harita_pin::Isaret> = sirali
            .iter()
            .enumerate()
            .map(|(sira, nokta)| harita_pin::Isaret {
                anahtar: anahtar(nokta),
                konum: (nokta.enlem, nokta.boylam),
                secili: self.otoyol_secili.as_deref() == Some(anahtar(nokta).as_str()),
                etiket: Some(format!("{}", sira + 1)),
            })
            .collect();
        let cizgi: Vec<(f64, f64)> = sirali
            .iter()
            .filter(|nokta| nokta.il != "Otoyol")
            .map(|nokta| (nokta.enlem, nokta.boylam))
            .collect();

        egui::SidePanel::left("otoyol_liste")
            .resizable(false)
            .exact_width(298.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14.0, 14.0)),
            )
            .show(ctx, |ui| {
                bolum_etiketi(ui, "O-5 · İZMİR – İSTANBUL");
                ui.label(
                    egui::RichText::new(format!("{} pin · çekici hattı", NOKTALAR.len()))
                        .size(10.0)
                        .color(DIM),
                );
                ui.add_space(6.0);
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Tüm pini göster")
                                .size(10.0)
                                .color(METIN),
                        )
                        .fill(KART)
                        .stroke(egui::Stroke::new(1.0_f32, KENAR))
                        .rounding(egui::Rounding::same(8.0)),
                    )
                    .clicked()
                {
                    sigdir = true;
                }
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .id_source("otoyol_pin_listesi")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (sira, nokta) in sirali.iter().enumerate() {
                            let anahtar = self::anahtar(nokta);
                            let secili_mi = self.otoyol_secili.as_deref() == Some(anahtar.as_str());
                            let metin =
                                egui::RichText::new(format!("{:>2}  {}", sira + 1, etiket(nokta)))
                                    .size(10.5)
                                    .color(if secili_mi { KOYU_YAZI } else { METIN });
                            let buton = egui::Button::new(metin)
                                .fill(if secili_mi { VURGU } else { KART })
                                .stroke(egui::Stroke::new(
                                    1.0_f32,
                                    if secili_mi { VURGU } else { KENAR },
                                ))
                                .rounding(egui::Rounding::same(8.0))
                                .min_size(egui::vec2(ui.available_width(), 26.0));
                            if ui.add(buton).clicked() {
                                yeni_secim = Some(anahtar);
                            }
                            ui.add_space(3.0);
                        }
                    });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ARKA_PLAN)
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0)),
            )
            .show(ctx, |ui| {
                bolum_etiketi(ui, "O-5 PIN HARİTASI");
                ui.label(
                    egui::RichText::new(
                        "Pinler otoyol hattı üzerinde sıralı; bir pine tıklayınca anahtar kelimeleri ve pin bağlantısı görünür.",
                    )
                    .size(10.0)
                    .color(DIM),
                );
                ui.add_space(8.0);
                let boyut = egui::vec2(ui.available_width(), 430.0);
                // İlk karede panel genişliği henüz bilinmez; sığdırma o zaman yapılmaz.
                if self.otoyol_sigdir && boyut.x > 200.0 {
                    self.otoyol_sigdir = false;
                    self.otoyol_harita
                        .sigdir_alana(&cizgi, boyut.x - 90.0, boyut.y - 90.0);
                }
                if let Some(harita_pin::Tiklama::Isaret(anahtar)) =
                    self.otoyol_harita
                        .goster_cizgi(ui, boyut, &isaretler, &cizgi, None)
                {
                    yeni_secim = Some(anahtar);
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Yakınlaştır +").size(10.0).color(METIN),
                            )
                            .fill(PANEL)
                            .stroke(egui::Stroke::new(1.0_f32, KENAR))
                            .rounding(egui::Rounding::same(8.0)),
                        )
                        .clicked()
                    {
                        yakinlas = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Uzaklaştır −").size(10.0).color(METIN),
                            )
                            .fill(PANEL)
                            .stroke(egui::Stroke::new(1.0_f32, KENAR))
                            .rounding(egui::Rounding::same(8.0)),
                        )
                        .clicked()
                    {
                        uzaklas = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Haritayı yenile").size(10.0).color(METIN),
                            )
                            .fill(PANEL)
                            .stroke(egui::Stroke::new(1.0_f32, KENAR))
                            .rounding(egui::Rounding::same(8.0)),
                        )
                        .clicked()
                    {
                        self.otoyol_harita.yenile();
                    }
                    if let Some(nokta) = secili {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · seçili pin: {}",
                                sektor,
                                etiket(nokta)
                            ))
                            .size(9.5)
                            .color(DIM),
                        );
                    }
                });
                ui.add_space(10.0);

                let Some(nokta) = secili else {
                    return;
                };
                egui::Frame::none()
                    .fill(KART)
                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                    .rounding(egui::Rounding::same(14.0))
                    .inner_margin(egui::Margin::symmetric(16.0, 14.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            egui::RichText::new(format!("PİN · {}", etiket(nokta)))
                                .size(9.0)
                                .color(DIM),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "{:.4}, {:.4}",
                                nokta.enlem, nokta.boylam
                            ))
                            .size(10.5)
                            .color(SOLUK),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("ANAHTAR KELİMELER")
                                .size(9.0)
                                .color(DIM),
                        );
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            for kelime in anahtar_kelimeler(&sektor, nokta) {
                                pill(ui, &kelime, YESIL);
                            }
                        });
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("PİN BAĞLANTISI")
                                .size(9.0)
                                .color(DIM),
                        );
                        ui.label(
                            egui::RichText::new(pin_link(&sektor, nokta))
                                .size(9.0)
                                .color(SOLUK),
                        );
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    [150.0, 30.0],
                                    egui::Button::new(
                                        egui::RichText::new("Bağlantıyı kopyala")
                                            .size(10.0)
                                            .color(KOYU_YAZI),
                                    )
                                    .fill(VURGU)
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                kopyala = true;
                            }
                            if ui
                                .add_sized(
                                    [130.0, 30.0],
                                    egui::Button::new(
                                        egui::RichText::new("Haritada aç").size(10.0).color(METIN),
                                    )
                                    .fill(PANEL)
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                ac = true;
                            }
                        });
                    });

                ui.add_space(10.0);
                ui.label(egui::RichText::new("PİN NASIL ATILIYOR").size(9.0).color(DIM));
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    for (baslik, aciklama) in adimlar() {
                        egui::Frame::none()
                            .fill(PANEL)
                            .stroke(egui::Stroke::new(1.0_f32, KENAR))
                            .rounding(egui::Rounding::same(10.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.set_width(210.0);
                                ui.label(
                                    egui::RichText::new(baslik).size(10.0).color(METIN).strong(),
                                );
                                ui.label(
                                    egui::RichText::new(aciklama).size(8.8).color(DIM),
                                );
                            });
                    }
                });
            });

        if let Some(secim) = yeni_secim {
            if let Some(nokta) = NOKTALAR.iter().find(|nokta| anahtar(nokta) == secim) {
                // Seçilen pin yakınlaştırılır: küme içindeki pinler ayrışır.
                self.otoyol_harita.ayarla(nokta.enlem, nokta.boylam, 10);
            }
            self.otoyol_secili = Some(secim);
        }
        if sigdir {
            self.otoyol_sigdir = true;
        }
        if yakinlas {
            self.otoyol_harita.yakinlas();
        }
        if uzaklas {
            self.otoyol_harita.uzaklas();
        }
        if kopyala {
            if let Some(nokta) = secili {
                let link = pin_link(&sektor, nokta);
                ctx.output_mut(|cikti| cikti.copied_text = link.clone());
                self.logs
                    .push(format!("[✓] Pin bağlantısı kopyalandı: {link}"));
            }
        }
        if ac {
            if let Some(nokta) = secili {
                let link = pin_link(&sektor, nokta);
                ctx.output_mut(|cikti| {
                    cikti.open_url = Some(egui::OpenUrl::new_tab(link));
                });
            }
        }
    }
}

/// İşletmenin kendi konumundan O-5 hattındaki en yakın pinlere uzaklık (km).
pub fn uzaklik_km(enlem1: f64, boylam1: f64, enlem2: f64, boylam2: f64) -> f64 {
    let r = 6371.0;
    let d_enlem = (enlem2 - enlem1).to_radians();
    let d_boylam = (boylam2 - boylam1).to_radians();
    let a = (d_enlem / 2.0).sin().powi(2)
        + enlem1.to_radians().cos() * enlem2.to_radians().cos() * (d_boylam / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn koridor_izmir_ve_istanbul_uclarini_icerir() {
        assert!(NOKTALAR.iter().any(|n| n.il == "İstanbul"));
        assert!(NOKTALAR.iter().any(|n| n.il == "İzmir"));
        assert!(NOKTALAR.iter().any(|n| n.ilce == "O-5 Otoyolu"));
        assert!(NOKTALAR.iter().any(|n| n.ilce == "Savaştepe"));
        assert!(NOKTALAR.len() >= 30);
    }

    #[test]
    fn her_pinin_sekiz_anahtar_kelimesi_var() {
        for nokta in NOKTALAR {
            let kelimeler = anahtar_kelimeler("Oto kurtarma", nokta);
            assert_eq!(kelimeler.len(), 8);
            assert_eq!(kelimeler.len(), HIZMETLER.len());
            assert!(kelimeler.iter().all(|k| !k.trim().is_empty()));
        }
    }

    #[test]
    fn hizmet_kelimeleri_istenen_listeyi_icerir() {
        let nokta = NOKTALAR
            .iter()
            .find(|n| n.ilce == "Saruhanlı")
            .expect("Saruhanlı pini");
        let kelimeler = anahtar_kelimeler("Oto kurtarma", nokta);
        assert_eq!(
            kelimeler,
            vec![
                "çekici",
                "oto kurtarma",
                "yol yardım 7/24",
                "en yakın çekici",
                "benzin destek",
                "mazot takviye",
                "akü takviye",
                "akücü",
            ]
        );
    }
    #[test]
    fn kelimelerde_bolge_adi_gecmez() {
        for nokta in NOKTALAR {
            let kelimeler = anahtar_kelimeler("Oto kurtarma", nokta);
            for kelime in &kelimeler {
                let kucuk = kelime.to_lowercase();
                assert!(!kucuk.contains("saruhanlı"), "ilçe eklenmiş: {kelime}");
                assert!(!kucuk.contains("akhisar"), "ilçe eklenmiş: {kelime}");
                assert!(!kucuk.contains("otoyolu"), "otoyol adı eklenmiş: {kelime}");
                assert!(
                    HIZMETLER.contains(&kelime.as_str()),
                    "beklenmeyen kelime: {kelime}"
                );
            }
        }
    }

    #[test]
    fn otoyol_pini_de_ayni_sekiz_kelimeyi_kullanir() {
        let otoyol = NOKTALAR
            .iter()
            .find(|n| n.ilce == "O-5 Otoyolu")
            .expect("O-5 pini olmalı");
        assert_eq!(
            anahtar_kelimeler("Oto kurtarma", otoyol),
            HIZMETLER
                .iter()
                .map(|h| (*h).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn pin_linki_ilceyi_kodlar() {
        let nokta = NOKTALAR
            .iter()
            .find(|n| n.ilce == "Saruhanlı")
            .expect("Saruhanlı pini");
        let link = pin_link("Oto kurtarma", nokta);
        assert!(link.starts_with("https://www.google.com/maps/search/?api=1&query="));
        assert!(link.contains("Saruhanl%C4%B1"));
    }

    #[test]
    fn adimlar_dort_adimdir() {
        let adimlar = adimlar();
        assert_eq!(adimlar.len(), 4);
        assert!(adimlar[0].0.starts_with('1'));
        assert!(adimlar[3].1.contains("Hizmet alanı") || adimlar[3].1.contains("hizmet alanı"));
    }

    #[test]
    fn uzaklik_manisa_izmir_arasi_makul() {
        let km = uzaklik_km(38.7350, 27.5650, 38.4700, 27.2200);
        assert!(km > 30.0 && km < 80.0, "beklenmeyen uzaklık: {km}");
    }
}
