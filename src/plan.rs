//! Otomatik üretim planı: işaretlenen işleri 10 saatlik takvime yayar ve
//! uygulama açıkken kendiliğinden çalıştırır.
//!
//! Plan diske yazılır (`plan.json`); uygulama kapanıp açılsa da kaldığı yerden
//! devam eder. Her görev, sırası gelen saatte tek seferde üretilir; elle
//! hiçbir düğmeye basmak gerekmez. Dış servis çağrılmaz, her şey yereldir.

use crate::*;
use eframe::egui;
use serde::{Deserialize, Serialize};

/// Planın tamamlanacağı süre (dakika): 10 saat.
pub const SURE_DK: u32 = 600;
/// Plan dosyası.
pub const DOSYA: &str = "plan.json";
const SURUM: u32 = 1;
/// Hizmet seçili değilse üretilen tanıtım dosyaları.
const TANITIM: [&str; 3] = ["gbp", "nap", "rapor"];

/// Şu anki unix saniyesi.
pub fn simdi() -> i64 {
    chrono::Local::now().timestamp()
}

/// Görevin durumu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Durum {
    #[default]
    Bekliyor,
    Bitti,
    Hata,
}

impl Durum {
    pub fn isaret(self) -> &'static str {
        match self {
            Durum::Bekliyor => "•",
            Durum::Bitti => "✓",
            Durum::Hata => "✕",
        }
    }

    pub fn etiket(self) -> &'static str {
        match self {
            Durum::Bekliyor => "sırada",
            Durum::Bitti => "üretildi",
            Durum::Hata => "hata",
        }
    }
}

/// Tek üretim görevi: bir müşteri için seçili hizmetlerin dosyaları.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gorev {
    pub anahtar: String,
    pub ad: String,
    /// Üretilecek hizmet kodları.
    pub hizmetler: Vec<String>,
    /// Sırası geldiği an (unix saniye).
    pub baslangic: i64,
    #[serde(default)]
    pub durum: Durum,
    #[serde(default)]
    pub bitis: Option<i64>,
    #[serde(default)]
    pub dosya: usize,
    #[serde(default)]
    pub not: String,
}

/// 10 saatlik üretim takvimi.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Plan {
    #[serde(default)]
    surum: u32,
    /// Planın başladığı an.
    #[serde(default)]
    pub baslangic: i64,
    /// Planın biteceği an.
    #[serde(default)]
    pub bitis: i64,
    /// Görevler arası saniye.
    #[serde(default)]
    pub aralik_sn: u64,
    /// Otomatik çalışma açık mı?
    #[serde(default)]
    pub calisiyor: bool,
    #[serde(default)]
    pub gorevler: Vec<Gorev>,
}

impl Plan {
    pub fn yukle() -> Self {
        let metin = fs::read_to_string(DOSYA).unwrap_or_default();
        Self::metinden(&metin)
    }

    pub fn metinden(metin: &str) -> Self {
        let mut plan: Plan = serde_json::from_str(metin).unwrap_or_default();
        plan.surum = SURUM;
        plan
    }

    pub fn kaydet(&self) {
        if let Ok(metin) = serde_json::to_string_pretty(self) {
            let _ = fs::write(DOSYA, metin);
        }
    }

    /// Biten görev sayısı.
    pub fn tamamlanan(&self) -> usize {
        self.gorevler
            .iter()
            .filter(|gorev| gorev.durum == Durum::Bitti)
            .count()
    }

    /// Üretilen toplam dosya.
    pub fn dosya_sayisi(&self) -> usize {
        self.gorevler.iter().map(|gorev| gorev.dosya).sum()
    }

    /// Tamamlanma yüzdesi (0-100).
    pub fn yuzde(&self) -> u32 {
        if self.gorevler.is_empty() {
            return 0;
        }
        (self.tamamlanan() * 100 / self.gorevler.len()) as u32
    }

    /// Bitişe kalan saniye.
    pub fn kalan_sn(&self, simdi: i64) -> i64 {
        (self.bitis - simdi).max(0)
    }

    /// Sıradaki görevin zamanı (yoksa None).
    pub fn sonraki(&self, simdi: i64) -> Option<&Gorev> {
        self.gorevler
            .iter()
            .filter(|gorev| gorev.durum == Durum::Bekliyor)
            .min_by_key(|gorev| gorev.baslangic)
            .map(|gorev| {
                let _ = simdi;
                gorev
            })
    }

    /// İşaretlenen bütün işleri 10 saate yayar.
    pub fn olustur(defter: &musteriler::Defter, gelir: &gelir::Gelir, simdi: i64) -> Plan {
        let mut gorevler: Vec<Gorev> = Vec::new();
        for musteri in defter.kayitlar() {
            if musteri.durum.kara_listede() {
                continue;
            }
            let hizmetler = hizmet_listesi(gelir.kayit(&musteri.anahtar));
            gorevler.push(Gorev {
                anahtar: musteri.anahtar.clone(),
                ad: musteri.ad.clone(),
                hizmetler,
                baslangic: simdi,
                durum: Durum::Bekliyor,
                bitis: None,
                dosya: 0,
                not: String::new(),
            });
        }
        let sure = SURE_DK as u64 * 60;
        let adet = gorevler.len().max(1) as u64;
        let aralik = (sure / adet).max(5);
        for (sira, gorev) in gorevler.iter_mut().enumerate() {
            gorev.baslangic = simdi + (sira as u64 * aralik) as i64;
        }
        Plan {
            surum: SURUM,
            baslangic: simdi,
            bitis: simdi + sure as i64,
            aralik_sn: aralik,
            calisiyor: true,
            gorevler,
        }
    }

    /// Sırası gelen görevleri çalıştırır; kaç görev işlendiğini döner.
    pub fn tick(
        &mut self,
        defter: &musteriler::Defter,
        gelir: &gelir::Gelir,
        logs: &mut Vec<String>,
        simdi: i64,
    ) -> usize {
        if !self.calisiyor || self.gorevler.is_empty() {
            return 0;
        }
        let gun = bugun_anahtari();
        let mut yapilan = 0;
        for sira in 0..self.gorevler.len() {
            if yapilan >= 3 {
                break;
            }
            if self.gorevler[sira].durum != Durum::Bekliyor || self.gorevler[sira].baslangic > simdi
            {
                continue;
            }
            let anahtar = self.gorevler[sira].anahtar.clone();
            let kodlar = self.gorevler[sira].hizmetler.clone();
            let Some(musteri) = defter.kayit(&anahtar).cloned() else {
                self.gorevler[sira].durum = Durum::Hata;
                self.gorevler[sira].not = "defterde yok".to_string();
                continue;
            };
            let kayit = match gelir.kayit(&anahtar) {
                Some(kayit) if kayit.aylik() > 0 => kayit.clone(),
                _ => gelir::MusteriGelir {
                    anahtar: anahtar.clone(),
                    ad: musteri.ad.clone(),
                    hizmetler: kodlar,
                    ..Default::default()
                },
            };
            match teslimat::yaz(&musteri, &kayit, &gun) {
                Ok(ciktilar) => {
                    self.gorevler[sira].durum = Durum::Bitti;
                    self.gorevler[sira].bitis = Some(simdi);
                    self.gorevler[sira].dosya = ciktilar.len();
                }
                Err(hata) => {
                    self.gorevler[sira].durum = Durum::Hata;
                    self.gorevler[sira].not = hata;
                }
            }
            yapilan += 1;
        }
        if yapilan > 0 {
            if self.tamamlanan() == self.gorevler.len() {
                self.calisiyor = false;
                logs.push(format!(
                    "[✓] Otomatik üretim bitti: {} görev, {} dosya.",
                    self.tamamlanan(),
                    self.dosya_sayisi()
                ));
            }
            self.kaydet();
        }
        yapilan
    }
}

/// Müşterinin satın aldığı hizmetler; yoksa tanıtım dosyaları.
fn hizmet_listesi(kayit: Option<&gelir::MusteriGelir>) -> Vec<String> {
    let mut kodlar: Vec<String> = Vec::new();
    if let Some(kayit) = kayit {
        match kayit.paket.as_deref().and_then(gelir::paket) {
            Some(paket) => kodlar.extend(paket.hizmetler.iter().map(|kod| kod.to_string())),
            None => kodlar.extend(kayit.hizmetler.iter().cloned()),
        }
        kodlar.extend(kayit.tek_seferler.iter().cloned());
    }
    if kodlar.is_empty() {
        kodlar.extend(TANITIM.iter().map(|kod| kod.to_string()));
    }
    kodlar
}

/// "9s 42d" biçiminde kalan süre.
pub fn sure_metni(saniye: i64) -> String {
    let dakika = (saniye.max(0) + 59) / 60;
    format!("{}s {}d", dakika / 60, dakika % 60)
}

impl LeadApp {
    /// Gelir katmanının "Plan" sayfası.
    pub(crate) fn plan_ekrani(&mut self, ui: &mut egui::Ui) {
        let simdi = simdi();
        let mut olustur = false;
        let mut duraklat = false;
        let mut sifirla = false;

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("10 SAATTE ÜRETİM")
                    .font(baslik_font(14.0))
                    .color(METIN),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{} görev · {} dosya · %{} bitti",
                    self.plan.gorevler.len(),
                    self.plan.dosya_sayisi(),
                    self.plan.yuzde()
                ))
                .size(10.5)
                .color(SOLUK),
            );
        });
        ui.add_space(4.0);
        ui.add(
            egui::ProgressBar::new(self.plan.yuzde() as f32 / 100.0)
                .desired_width((ui.available_width() - 8.0).max(120.0))
                .fill(VURGU),
        );
        ui.add_space(4.0);
        if self.plan.gorevler.is_empty() {
            ui.label(
                egui::RichText::new(
                    "Henüz plan yok: \"Tümünü planla\" işaretli bütün işleri 10 saate yayar.",
                )
                .size(10.5)
                .color(DIM),
            );
        } else {
            ui.label(
                egui::RichText::new(format!(
                    "{} · {}/{} · kalan {} · bitiş {}",
                    if self.plan.calisiyor {
                        "çalışıyor"
                    } else {
                        "durdu"
                    },
                    self.plan.tamamlanan(),
                    self.plan.gorevler.len(),
                    sure_metni(self.plan.kalan_sn(simdi)),
                    chrono::DateTime::from_timestamp(self.plan.bitis, 0)
                        .map(|an| an.format("%H:%M").to_string())
                        .unwrap_or_else(|| "—".to_string())
                ))
                .size(10.0)
                .color(SOLUK),
            );
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if birincil_buton(ui, "Tümünü planla · 10 saat", true, None).clicked() {
                olustur = true;
            }
            ui.add_space(6.0);
            if ikincil_buton(
                ui,
                if self.plan.calisiyor {
                    "Duraklat"
                } else {
                    "Devam et"
                },
                !self.plan.gorevler.is_empty(),
                None,
            )
            .clicked()
            {
                duraklat = true;
            }
            ui.add_space(6.0);
            if hayalet_buton(ui, "Planı sil").clicked() {
                sifirla = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "görev aralığı {} sn · her görev kendi saatinde çalışır",
                        self.plan.aralik_sn
                    ))
                    .size(9.0)
                    .color(DIM),
                );
            });
        });
        ui.add_space(8.0);

        if !self.plan.gorevler.is_empty() {
            bolum_etiketi(ui, "SIRADAKİ GÖREVLER");
            ui.add_space(2.0);
            egui::ScrollArea::vertical()
                .id_source("plan_gorevler")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for gorev in self.plan.gorevler.iter().take(200) {
                        ui.horizontal(|ui| {
                            let renk = match gorev.durum {
                                Durum::Bitti => YESIL,
                                Durum::Hata => KIRMIZI,
                                Durum::Bekliyor => DIM,
                            };
                            ui.label(
                                egui::RichText::new(gorev.durum.isaret())
                                    .size(11.0)
                                    .color(renk),
                            );
                            ui.label(egui::RichText::new(&gorev.ad).size(10.5).color(METIN));
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} hizmet · {}",
                                    gorev.hizmetler.len(),
                                    gorev.durum.etiket()
                                ))
                                .size(9.5)
                                .color(DIM),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if gorev.durum == Durum::Bitti {
                                        ui.label(
                                            egui::RichText::new(format!("{} dosya", gorev.dosya))
                                                .size(9.5)
                                                .color(YESIL),
                                        );
                                    } else if gorev.durum == Durum::Hata {
                                        ui.label(
                                            egui::RichText::new(&gorev.not)
                                                .size(9.5)
                                                .color(KIRMIZI),
                                        );
                                    } else {
                                        ui.label(
                                            egui::RichText::new(
                                                chrono::DateTime::from_timestamp(
                                                    gorev.baslangic,
                                                    0,
                                                )
                                                .map(|an| an.format("%H:%M").to_string())
                                                .unwrap_or_default(),
                                            )
                                            .size(9.5)
                                            .color(DIM),
                                        );
                                    }
                                },
                            );
                        });
                    }
                });
        }

        if olustur {
            self.plan = Plan::olustur(&self.defter, &self.gelir, simdi);
            self.plan.kaydet();
            self.logs.push(format!(
                "[✓] Plan kuruldu: {} müşteri, 10 saatte bitecek; ilk görev hemen çalışır.",
                self.plan.gorevler.len()
            ));
        }
        if duraklat {
            self.plan.calisiyor = !self.plan.calisiyor;
            self.plan.kaydet();
        }
        if sifirla {
            self.plan = Plan::default();
            self.plan.kaydet();
            self.logs.push("[i] Plan silindi.".to_string());
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn defter_iki() -> musteriler::Defter {
        let mut defter = musteriler::Defter::default();
        for (sira, ad) in ["Bir", "İki"].iter().enumerate() {
            defter.isle(
                &format!("osm:node/{sira}"),
                ad,
                "0532 123 45 67",
                "Manisa",
                "Kafe",
                "osm",
                "yok",
                "",
                40,
                "2026-11-01",
            );
        }
        defter
    }

    #[test]
    fn plan_on_saate_yayilir() {
        let defter = defter_iki();
        let gelir = gelir::Gelir::metinden("");
        let plan = Plan::olustur(&defter, &gelir, 1_000);
        assert_eq!(plan.gorevler.len(), 2);
        assert_eq!(plan.bitis - plan.baslangic, 600 * 60);
        assert!(plan.aralik_sn >= 5);
        assert_eq!(plan.gorevler[0].baslangic, 1_000);
        assert!(plan.calisiyor);
        // Hizmet seçilmemişse tanıtım dosyaları üretilir.
        assert_eq!(plan.gorevler[0].hizmetler.len(), 3);
    }

    #[test]
    fn abone_paketinin_hizmetleri_kullanilir() {
        let gelir = gelir::Gelir::metinden(
            "{\"kayitlar\":[{\"anahtar\":\"osm:node/0\",\"ad\":\"Bir\",\"paket\":\"buyume\",\"paket_fiyati\":5000}]}",
        );
        let liste = hizmet_listesi(gelir.kayit("osm:node/0"));
        assert!(liste.contains(&"gbp".to_string()));
        assert!(liste.len() >= 5);
        assert!(hizmet_listesi(None).contains(&"rapor".to_string()));
    }

    #[test]
    fn yuzde_ve_kalan_sure_hesaplanir() {
        let defter = defter_iki();
        let gelir = gelir::Gelir::metinden("");
        let mut plan = Plan::olustur(&defter, &gelir, 0);
        assert_eq!(plan.yuzde(), 0);
        plan.gorevler[0].durum = Durum::Bitti;
        assert_eq!(plan.tamamlanan(), 1);
        assert_eq!(plan.yuzde(), 50);
        assert_eq!(plan.kalan_sn(600 * 60 - 60), 60);
        assert_eq!(sure_metni(60), "0s 1d");
        assert_eq!(sure_metni(3_600), "1s 0d");
    }
}
