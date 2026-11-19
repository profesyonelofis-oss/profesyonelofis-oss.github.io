//! Harita pini: işletmenin gerçek konumunu Google Maps'te işaretleme.
//!
//! Uygulama sahte işletme ya da kopya pin üretmez. Yalnızca işletmenin **kendi**
//! konumunu Google Maps pin bağlantısına çevirir, işletme sahibine gönderilecek
//! metni hazırlar ve pinin durumunu `pinler.json` dosyasında tutar
//! (pin yok / pin hazır / onaylandı). Kayıt işletmenin adını, adresini,
//! telefonunu ve koordinatını da saklar; böylece uygulama yeniden açıldığında
//! liste kaybolmaz.
//!
//! Pini haritaya ekleme adımı Google hesabıyla yapılır; uygulama o adımı
//! taklit etmez, bağlantıyı ve metni hazırlar.

use crate::*;
use eframe::egui;
use std::collections::BTreeMap;
use std::fs;

/// Pin durumlarının yazıldığı dosya.
pub const DOSYA: &str = "pinler.json";

/// Tek işletmenin pin durumu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Durum {
    /// Henüz pin hazırlanmadı.
    #[default]
    Yok,
    /// Pin bağlantısı hazır, sahibine gönderilecek.
    Hazir,
    /// Sahip konumu doğruladı.
    Onaylandi,
}

impl Durum {
    pub fn etiket(self) -> &'static str {
        match self {
            Durum::Yok => "pin yok",
            Durum::Hazir => "pin hazır",
            Durum::Onaylandi => "onaylandı",
        }
    }

    fn renk(self) -> egui::Color32 {
        match self {
            Durum::Yok => DIM,
            Durum::Hazir => TURUNCU,
            Durum::Onaylandi => YESIL,
        }
    }
}

/// Diske yazılan tek pin kaydı: işletmenin kimliği ve konumu.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Pin {
    #[serde(default)]
    pub durum: Durum,
    /// Durumun değiştiği gün (`YYYY-MM-DD`).
    #[serde(default)]
    pub tarih: String,
    #[serde(default)]
    pub ad: String,
    #[serde(default)]
    pub adres: String,
    #[serde(default)]
    pub telefon: String,
    /// Posta kodu; boşsa şemaya yazılmaz.
    #[serde(default)]
    pub posta: String,
    #[serde(default)]
    pub enlem: f64,
    #[serde(default)]
    pub boylam: f64,
}

impl Pin {
    pub fn konum(&self) -> (f64, f64) {
        (self.enlem, self.boylam)
    }
}

/// Kalıcı pin defteri: işletme anahtarı → pin kaydı.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Pinler {
    #[serde(default)]
    pub kayitlar: BTreeMap<String, Pin>,
}

impl Pinler {
    /// Dosyayı okur; dosya yoksa ya da bozuksa boş defter döner.
    pub fn yukle() -> Self {
        fs::read_to_string(DOSYA)
            .ok()
            .and_then(|metin| serde_json::from_str(&metin).ok())
            .unwrap_or_default()
    }

    pub fn kaydet(&self) {
        if let Ok(metin) = serde_json::to_string_pretty(self) {
            let _ = fs::write(DOSYA, metin);
        }
    }

    pub fn durum(&self, anahtar: &str) -> Durum {
        self.kayitlar
            .get(anahtar)
            .map(|pin| pin.durum)
            .unwrap_or_default()
    }

    /// Kaydı yazar (varsa üzerine).
    pub fn yaz(&mut self, anahtar: &str, pin: Pin) {
        self.kayitlar.insert(anahtar.to_string(), pin);
    }

    /// Kaydı siler; işaret geri alınır.
    pub fn sil(&mut self, anahtar: &str) {
        self.kayitlar.remove(anahtar);
    }

    pub fn say(&self, durum: Durum) -> usize {
        self.kayitlar
            .values()
            .filter(|pin| pin.durum == durum)
            .count()
    }
}

/// İşletme konumunun paylaşılabilir Google Maps pin bağlantısı.
pub fn baglanti(enlem: f64, boylam: f64) -> String {
    format!("https://www.google.com/maps/search/?api=1&query={enlem:.6},{boylam:.6}")
}

/// İşletme sahibine gönderilecek pin metni.
pub fn paylas_metni(ad: &str, adres: &str, enlem: f64, boylam: f64) -> String {
    let adres = adres.trim();
    let konum = if adres.is_empty() {
        String::new()
    } else {
        format!("\n{adres}")
    };
    format!(
        "Merhaba, {ad} için Google Haritalar konum pinini hazırladım.{konum}\n\n{}\n\nBağlantıya dokunup konumun doğru olduğunu görebilirsiniz. Haritada nokta doğruysa \"konum doğru\" yazmanız yeterli; kayıtlı adres farklıysa düzeltelim.",
        baglanti(enlem, boylam)
    )
}

/// Listede gösterilecek tek satır: kayıtlı pin ya da taranmış işletme.
#[derive(Clone)]
struct Satir {
    anahtar: String,
    ad: String,
    adres: String,
    telefon: String,
    konum: (f64, f64),
}

impl LeadApp {
    pub(crate) fn pin_ekrani(&mut self, ctx: &egui::Context) {
        let mut secim: Option<String> = None;
        let mut haritada = false;
        let mut link_kopyala = false;
        let mut whatsapp = false;
        let mut yeni_durum: Option<Durum> = None;
        let mut toplu_kopyala = false;
        let mut taslak_secim: Option<(f64, f64)> = None;
        let mut konum_kaydet = false;
        let mut taslak_birak = false;
        let mut yakinlas = false;
        let mut uzaklas = false;
        let mut harita_yenile = false;
        let mut tumunu_sigdir = false;
        let mut harita_secim: Option<String> = None;

        // Çizim sırasında `self` ödünç alınmasın: liste, arama ve durumlar kopyalanır.
        let durumlar: BTreeMap<String, Durum> = self
            .pinler
            .kayitlar
            .iter()
            .map(|(anahtar, pin)| (anahtar.clone(), pin.durum))
            .collect();
        let mut arama = std::mem::take(&mut self.pin_arama);
        // Önce kayıtlı pinler, sonra henüz pinlenmemiş tarama sonuçları.
        let mut items: Vec<Satir> = self
            .pinler
            .kayitlar
            .iter()
            .map(|(anahtar, pin)| Satir {
                anahtar: anahtar.clone(),
                ad: pin.ad.clone(),
                adres: pin.adres.clone(),
                telefon: pin.telefon.clone(),
                konum: pin.konum(),
            })
            .collect();
        for lead in &self.leads {
            let Some(konum) = lead.konum else {
                continue;
            };
            let anahtar = musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum);
            if self.pinler.kayitlar.contains_key(&anahtar) {
                continue;
            }
            items.push(Satir {
                anahtar,
                ad: lead.name.clone(),
                adres: lead.address.clone(),
                telefon: lead.phone.clone(),
                konum,
            });
        }
        items.sort_by(|a, b| a.ad.to_lowercase().cmp(&b.ad.to_lowercase()));
        // Seçim yoksa ya da seçili kayıt listeden düştüyse ilk işletme seçilir:
        // kart hemen görünür, kullanıcı ekstra tıklamaz.
        let gecerli = self
            .pin_secili
            .as_ref()
            .map(|anahtar| items.iter().any(|satir| &satir.anahtar == anahtar))
            .unwrap_or(false);
        if !gecerli {
            self.pin_secili = items.first().map(|satir| satir.anahtar.clone());
        }
        let hazir = durumlar.values().filter(|d| **d == Durum::Hazir).count();
        let onayli = durumlar
            .values()
            .filter(|d| **d == Durum::Onaylandi)
            .count();
        let secili = self
            .pin_secili
            .as_ref()
            .and_then(|anahtar| items.iter().find(|satir| &satir.anahtar == anahtar))
            .cloned();
        let secili_durum = secili
            .as_ref()
            .map(|satir| durumlar.get(&satir.anahtar).copied().unwrap_or_default())
            .unwrap_or_default();
        let secili_tarih = secili
            .as_ref()
            .and_then(|satir| self.pinler.kayitlar.get(&satir.anahtar))
            .map(|pin| pin.tarih.clone())
            .unwrap_or_default();

        let isaretler: Vec<harita_pin::Isaret> = items
            .iter()
            .map(|satir| harita_pin::Isaret {
                anahtar: satir.anahtar.clone(),
                konum: satir.konum,
                secili: self.pin_secili.as_deref() == Some(satir.anahtar.as_str()),
                etiket: None,
            })
            .collect();
        // İlk açılışta harita tüm pinleri kapsar; sonra seçim değişince odaklanır.
        let odak = self.pin_odak.clone();
        if std::mem::take(&mut self.pin_sigdir) {
            let noktalar: Vec<(f64, f64)> = items.iter().map(|satir| satir.konum).collect();
            if !noktalar.is_empty() {
                self.harita.sigdir(&noktalar);
            }
            self.pin_odak = self.pin_secili.clone();
        } else if self.pin_secili != odak {
            if let Some(satir) = &secili {
                self.harita.yonlendir(satir.konum.0, satir.konum.1);
            }
            self.pin_odak = self.pin_secili.clone();
        }
        let taslak = self.pin_taslak;

        // --- Sol sütun: işletmeler -----------------------------------------
        egui::SidePanel::left("pin_liste")
            .resizable(false)
            .exact_width(298.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14.0, 14.0)),
            )
            .show(ctx, |ui| {
                bolum_etiketi(ui, "HARİTA PİNİ");
                ui.label(
                    egui::RichText::new(
                        "İşletmenin kendi konumu Google Maps pinine çevrilir; bağlantıyı sahibine yollarsınız.",
                    )
                    .size(9.5)
                    .color(DIM),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    pill(ui, &format!("{} işletme", items.len()), DIM);
                    pill(ui, &format!("{hazir} hazır"), TURUNCU);
                    pill(ui, &format!("{onayli} onaylı"), YESIL);
                });
                ui.add_space(8.0);
                ui.add(
                    egui::TextEdit::singleline(&mut arama)
                        .hint_text("İşletme ara")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);
                if ui
                    .add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new(
                            egui::RichText::new("Tüm pin linklerini kopyala")
                                .size(10.0)
                                .color(METIN),
                        )
                        .fill(KART)
                        .stroke(egui::Stroke::new(1.0_f32, KENAR))
                        .rounding(egui::Rounding::same(8.0)),
                    )
                    .clicked()
                {
                    toplu_kopyala = true;
                }
                ui.add_space(8.0);

                let suzgec = arama.trim().to_lowercase();
                egui::ScrollArea::vertical()
                    .id_source("pin_isletme_listesi")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if items.is_empty() {
                            ui.label(
                                egui::RichText::new(
                                    "Koordinatlı işletme yok. Müşteri Bulma'da tarama yapın; gelen kayıtlar burada listelenir.",
                                )
                                .size(10.0)
                                .color(DIM),
                            );
                        }
                        for satir in &items {
                            if !suzgec.is_empty()
                                && !satir.ad.to_lowercase().contains(&suzgec)
                                && !satir.adres.to_lowercase().contains(&suzgec)
                            {
                                continue;
                            }
                            let secili_mi =
                                self.pin_secili.as_deref() == Some(satir.anahtar.as_str());
                            let durum = durumlar.get(&satir.anahtar).copied().unwrap_or_default();
                            let metin =
                                format!("{}\n{}  ·  {}", satir.ad, durum.etiket(), satir.adres);
                            let buton = egui::Button::new(
                                egui::RichText::new(metin)
                                    .size(10.5)
                                    .color(if secili_mi { KOYU_YAZI } else { METIN }),
                            )
                            .fill(if secili_mi { VURGU } else { KART })
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                if secili_mi { VURGU } else { KENAR },
                            ))
                            .rounding(egui::Rounding::same(9.0))
                            .min_size(egui::vec2(ui.available_width(), 40.0));
                            if ui.add(buton).clicked() {
                                secim = Some(satir.anahtar.clone());
                            }
                            ui.add_space(4.0);
                        }
                    });
            });

        // --- Sağ sütun: seçili işletmenin pini ------------------------------
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ARKA_PLAN)
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0)),
            )
            .show(ctx, |ui| {
                let Some(satir) = secili.as_ref() else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("Soldan işletme seçin.")
                                .size(12.0)
                                .color(DIM),
                        );
                    });
                    return;
                };
                let (enlem, boylam) = satir.konum;
                egui::Frame::none()
                    .fill(KART)
                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                    .rounding(egui::Rounding::same(14.0))
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        bolum_etiketi(ui, "SEÇİLİ İŞLETME");
                        ui.label(
                            egui::RichText::new(&satir.ad)
                                .size(17.0)
                                .color(METIN)
                                .strong(),
                        );
                        ui.add_space(2.0);
                        if satir.adres.trim().is_empty() {
                            ui.label(
                                egui::RichText::new("Adres kayıtta yok")
                                    .size(10.0)
                                    .color(TURUNCU),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new(&satir.adres)
                                    .size(10.5)
                                    .color(SOLUK),
                            );
                        }
                        ui.label(
                            egui::RichText::new(format!("{enlem:.6}, {boylam:.6}"))
                                .size(10.0)
                                .color(DIM),
                        );
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            pill(ui, secili_durum.etiket(), secili_durum.renk());
                            if !secili_tarih.is_empty() {
                                pill(ui, &secili_tarih, DIM);
                            }
                        });
                        ui.add_space(10.0);

                        // --- Harita: işletme pini ve konum düzeltme ----------
                        let harita_boyut = egui::vec2(ui.available_width(), 300.0);
                        match self.harita.goster(ui, harita_boyut, &isaretler, taslak) {
                            Some(harita_pin::Tiklama::Isaret(anahtar)) => {
                                harita_secim = Some(anahtar);
                            }
                            Some(harita_pin::Tiklama::Konum(nokta)) => taslak_secim = Some(nokta),
                            None => {}
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
                                        egui::RichText::new("Haritayı yenile")
                                            .size(10.0)
                                            .color(METIN),
                                    )
                                    .fill(PANEL)
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                harita_yenile = true;
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Tüm pinleri göster")
                                            .size(10.0)
                                            .color(METIN),
                                    )
                                    .fill(PANEL)
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                tumunu_sigdir = true;
                            }
                            match taslak {
                                Some((e, b)) => {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Haritada seçilen konum: {e:.6}, {b:.6}"
                                        ))
                                        .size(9.5)
                                        .color(TURUNCU),
                                    );
                                }
                                None => {
                                    ui.label(
                                        egui::RichText::new(
                                            "Haritaya tıklayarak konumu düzeltebilirsiniz.",
                                        )
                                        .size(9.5)
                                        .color(DIM),
                                    );
                                }
                            }
                        });
                        if taslak.is_some() {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Konumu kaydet")
                                                .size(10.0)
                                                .color(KOYU_YAZI),
                                        )
                                        .fill(VURGU)
                                        .rounding(egui::Rounding::same(8.0)),
                                    )
                                    .clicked()
                                {
                                    konum_kaydet = true;
                                }
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Vazgeç")
                                                .size(10.0)
                                                .color(DIM),
                                        )
                                        .fill(PANEL)
                                        .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                        .rounding(egui::Rounding::same(8.0)),
                                    )
                                    .clicked()
                                {
                                    taslak_birak = true;
                                }
                                ui.label(
                                    egui::RichText::new(
                                        "Yeni konum kaydedilince pin bağlantısı ve WhatsApp metni güncellenir.",
                                    )
                                    .size(9.0)
                                    .color(DIM),
                                );
                            });
                        }
                        ui.add_space(10.0);

                        let genislik = if ui.available_width() > 620.0 {
                            180.0
                        } else {
                            (ui.available_width() - 18.0) / 3.0
                        };
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    [genislik, 30.0],
                                    egui::Button::new(
                                        egui::RichText::new("Haritada aç")
                                            .size(10.5)
                                            .color(KOYU_YAZI),
                                    )
                                    .fill(VURGU)
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                haritada = true;
                            }
                            if ui
                                .add_sized(
                                    [genislik, 30.0],
                                    egui::Button::new(
                                        egui::RichText::new("Pin linkini kopyala")
                                            .size(10.5)
                                            .color(METIN),
                                    )
                                    .fill(PANEL)
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                link_kopyala = true;
                            }
                            if ui
                                .add_sized(
                                    [genislik, 30.0],
                                    egui::Button::new(
                                        egui::RichText::new("WhatsApp'ta gönder")
                                            .size(10.5)
                                            .color(METIN),
                                    )
                                    .fill(PANEL)
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                whatsapp = true;
                            }
                        });
                        ui.add_space(10.0);

                        egui::Frame::none()
                            .fill(PANEL)
                            .rounding(egui::Rounding::same(9.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 9.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(
                                    egui::RichText::new(baglanti(enlem, boylam))
                                        .size(10.0)
                                        .color(SOLUK),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(paylas_metni(
                                        &satir.ad,
                                        &satir.adres,
                                        enlem,
                                        boylam,
                                    ))
                                    .size(10.0)
                                    .color(METIN),
                                );
                            });
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Pin hazır olarak işaretle")
                                            .size(10.0)
                                            .color(if secili_durum == Durum::Hazir {
                                                KOYU_YAZI
                                            } else {
                                                METIN
                                            }),
                                    )
                                    .fill(if secili_durum == Durum::Hazir {
                                        VURGU
                                    } else {
                                        PANEL
                                    })
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                yeni_durum = Some(Durum::Hazir);
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Sahip onayladı")
                                            .size(10.0)
                                            .color(if secili_durum == Durum::Onaylandi {
                                                KOYU_YAZI
                                            } else {
                                                METIN
                                            }),
                                    )
                                    .fill(if secili_durum == Durum::Onaylandi {
                                        YESIL
                                    } else {
                                        PANEL
                                    })
                                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                    .rounding(egui::Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                yeni_durum = Some(Durum::Onaylandi);
                            }
                            if secili_durum != Durum::Yok
                                && ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Geri al")
                                                .size(10.0)
                                                .color(DIM),
                                        )
                                        .fill(PANEL)
                                        .stroke(egui::Stroke::new(1.0_f32, KENAR))
                                        .rounding(egui::Rounding::same(8.0)),
                                    )
                                    .clicked()
                            {
                                yeni_durum = Some(Durum::Yok);
                            }
                        });
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(
                                "Pini kendi işletmesinin sahibi Google Haritalar'da doğrular. Sahte işletme/pin üretilmez; aynı noktaya ikinci kayıt eklemek Google kurallarına aykırıdır.",
                            )
                            .size(9.0)
                            .color(DIM),
                        );
                    });
            });

        // --- Eylemleri uygula -----------------------------------------------
        self.pin_arama = arama;
        if yakinlas {
            self.harita.yakinlas();
        }
        if uzaklas {
            self.harita.uzaklas();
        }
        if tumunu_sigdir {
            let noktalar: Vec<(f64, f64)> = items.iter().map(|satir| satir.konum).collect();
            if !noktalar.is_empty() {
                self.harita.sigdir(&noktalar);
            }
            self.pin_odak = self.pin_secili.clone();
        }
        if let Some(anahtar) = harita_secim {
            self.pin_secili = Some(anahtar);
            self.pin_taslak = None;
        }
        if harita_yenile {
            self.harita.yenile();
        }
        if let Some(nokta) = taslak_secim {
            self.pin_taslak = Some(nokta);
            self.logs.push(format!(
                "[i] Haritada seçildi: {:.6}, {:.6} — kaydetmek için 'Konumu kaydet'.",
                nokta.0, nokta.1
            ));
        }
        if taslak_birak {
            self.pin_taslak = None;
        }
        if let Some(anahtar) = secim {
            self.pin_secili = Some(anahtar);
            self.pin_taslak = None;
        }
        let Some(satir) = secili else {
            return;
        };
        let (enlem, boylam) = satir.konum;
        if konum_kaydet {
            if let Some((yeni_enlem, yeni_boylam)) = self.pin_taslak {
                let durum = if secili_durum == Durum::Yok {
                    Durum::Hazir
                } else {
                    secili_durum
                };
                // Posta kodu kayıtta varsa korunur (şemada kullanılıyor).
                let posta = self
                    .pinler
                    .kayitlar
                    .get(&satir.anahtar)
                    .map(|pin| pin.posta.clone())
                    .unwrap_or_default();
                self.pinler.yaz(
                    &satir.anahtar,
                    Pin {
                        durum,
                        tarih: bugun_anahtari(),
                        ad: satir.ad.clone(),
                        adres: satir.adres.clone(),
                        telefon: satir.telefon.clone(),
                        posta,
                        enlem: yeni_enlem,
                        boylam: yeni_boylam,
                    },
                );
                self.pinler.kaydet();
                self.harita.yonlendir(yeni_enlem, yeni_boylam);
                self.pin_taslak = None;
                self.logs.push(format!(
                    "[✓] {}: pin konumu güncellendi ({yeni_enlem:.6}, {yeni_boylam:.6}).",
                    satir.ad
                ));
            }
        }
        let link = baglanti(enlem, boylam);
        let metin = paylas_metni(&satir.ad, &satir.adres, enlem, boylam);
        if haritada {
            match tarayicida_ac(&link) {
                Ok(()) => self
                    .logs
                    .push(format!("[✓] {}: harita pini tarayıcıda açıldı.", satir.ad)),
                Err(hata) => self.logs.push(format!("[!] {hata}")),
            }
        }
        if link_kopyala {
            ctx.output_mut(|cikti| cikti.copied_text = link.clone());
            self.logs
                .push(format!("[✓] {}: pin linki panoya kopyalandı.", satir.ad));
        }
        if whatsapp {
            match whatsapp_numarasi(&satir.telefon) {
                Some(numara) => {
                    if sohbet_ac(&numara, &metin, &mut self.logs) {
                        self.logs.push(format!(
                            "[✓] {}: pin mesajı WhatsApp'ta açıldı (gönderme elle).",
                            satir.ad
                        ));
                    }
                }
                None => {
                    ctx.output_mut(|cikti| cikti.copied_text = metin);
                    self.logs.push(format!(
                        "[i] {}: telefon yok, pin mesajı panoya kopyalandı.",
                        satir.ad
                    ));
                }
            }
        }
        if toplu_kopyala {
            let liste: Vec<String> = items
                .iter()
                .map(|satir| format!("{}\t{}", satir.ad, baglanti(satir.konum.0, satir.konum.1)))
                .collect();
            if liste.is_empty() {
                self.logs
                    .push("[i] Kopyalanacak pin yok: koordinatlı işletme bulunamadı.".to_string());
            } else {
                let toplam = liste.len();
                ctx.output_mut(|cikti| cikti.copied_text = liste.join("\n"));
                self.logs.push(format!(
                    "[✓] {toplam} işletmenin pin linki panoya kopyalandı."
                ));
            }
        }
        if let Some(durum) = yeni_durum {
            if durum == Durum::Yok {
                self.pinler.sil(&satir.anahtar);
            } else {
                // Posta kodu kayıtta varsa korunur (şemada kullanılıyor).
                let posta = self
                    .pinler
                    .kayitlar
                    .get(&satir.anahtar)
                    .map(|pin| pin.posta.clone())
                    .unwrap_or_default();
                self.pinler.yaz(
                    &satir.anahtar,
                    Pin {
                        durum,
                        tarih: bugun_anahtari(),
                        ad: satir.ad.clone(),
                        adres: satir.adres.clone(),
                        telefon: satir.telefon.clone(),
                        posta,
                        enlem,
                        boylam,
                    },
                );
            }
            self.pinler.kaydet();
            self.logs.push(format!(
                "[✓] {}: pin durumu → {}.",
                satir.ad,
                durum.etiket()
            ));
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn baglanti_koordinati_tasir() {
        let link = baglanti(38.612345, 27.432109);
        assert!(link.starts_with("https://www.google.com/maps/search/?api=1&query="));
        assert!(link.contains("38.612345,27.432109"));
    }

    #[test]
    fn paylas_metni_pini_ve_adresi_icerir() {
        let metin = paylas_metni("Prime Garaj", "Adakale Mah. No:3", 38.61, 27.43);
        assert!(metin.contains("Prime Garaj"));
        assert!(metin.contains("Adakale Mah. No:3"));
        assert!(metin.contains("query=38.610000,27.430000"));
    }

    #[test]
    fn adres_bosken_bos_satir_yok() {
        let metin = paylas_metni("Dükkan", "   ", 38.0, 27.0);
        assert!(!metin.contains("\n\n\n"));
    }

    #[test]
    fn pin_yazilir_ve_geri_alinir() {
        let mut pinler = Pinler::default();
        assert_eq!(pinler.durum("a"), Durum::Yok);
        pinler.yaz(
            "a",
            Pin {
                durum: Durum::Hazir,
                tarih: "2026-11-10".to_string(),
                ad: "Dükkan".to_string(),
                enlem: 38.0,
                boylam: 27.0,
                ..Pin::default()
            },
        );
        assert_eq!(pinler.durum("a"), Durum::Hazir);
        assert_eq!(pinler.say(Durum::Hazir), 1);
        pinler.sil("a");
        assert_eq!(pinler.durum("a"), Durum::Yok);
        assert_eq!(pinler.say(Durum::Hazir), 0);
    }

    #[test]
    fn kayit_metinden_geri_okunur() {
        let metin = r#"{"kayitlar":{"k|1":{"durum":"Onaylandi","tarih":"2026-11-10","ad":"Dükkan","adres":"Cad. No:1","telefon":"0532","enlem":38.5,"boylam":27.5}}}"#;
        let pinler: Pinler = serde_json::from_str(metin).expect("pin defteri okunmalı");
        let pin = pinler.kayitlar.get("k|1").expect("kayıt bulunmalı");
        assert_eq!(pin.durum, Durum::Onaylandi);
        assert_eq!(pin.konum(), (38.5, 27.5));
    }
}
