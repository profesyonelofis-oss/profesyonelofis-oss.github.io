//! Günlük satış otomasyonu: 200.000 TL/ay hedefini günlük işe çevirir.
//!
//! Bu modül dış servis kullanmaz; her şey yerelde ve ücretsizdir. Yaptığı iş:
//!
//! - Hedefi matematikle günlük kota'ya indirir (ortalama paket → gereken abone
//!   → günlük mesaj).
//! - Defterdeki kayıtlardan **bugün** yazılacakları sıraya koyar: sıcak
//!   görüşmeler önce, takip zamanı gelenler sonra, hiç yazılmamışlar en sonda.
//! - Abonelere ait aylık teslim dosyalarının borcunu takip eder.
//!
//! Mesaj gönderimi bilerek otomatik değildir: WhatsApp düğmesi sohbeti hazır
//! metinle açar, "gönderdim" işaretlenince defter güncellenir.

use crate::*;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// Aylık gelir hedefi (TL).
pub const HEDEF_TL: u32 = 200_000;
/// Otomasyon defterinin dosyası.
pub const DOSYA: &str = "otomasyon.json";
/// Günlük mesaj kotası (WhatsApp freniyle aynı).
pub const GUNLUK_MESAJ: usize = 20;
/// Ayda çalışılan gün sayısı.
pub const ISI_GUNU: u32 = 26;
/// Muhafazakâr kapanış oranı: 100 mesajdan 4'ü aboneye döner.
pub const KAPANMA_YUZDE: u32 = 4;
/// Takip mesajı için geçmesi gereken gün.
pub const TAKIP_GUN: i64 = 4;
const SURUM: u32 = 1;

/// Yazılacak kişinin durumu; sıralamayı belirler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsTuru {
    /// İşletme ilgilendi: en önce.
    Sicak,
    /// Daha önce yazıldı, takip zamanı geldi.
    Takip,
    /// Hiç temas kurulmadı.
    YeniMesaj,
}

impl IsTuru {
    pub fn etiket(self) -> &'static str {
        match self {
            IsTuru::Sicak => "sıcak",
            IsTuru::Takip => "takip",
            IsTuru::YeniMesaj => "ilk mesaj",
        }
    }

    fn oncelik(self) -> u8 {
        match self {
            IsTuru::Sicak => 0,
            IsTuru::Takip => 1,
            IsTuru::YeniMesaj => 2,
        }
    }
}

/// Bugün yazılacak tek kişi.
#[derive(Clone, Debug)]
pub struct Is {
    pub tur: IsTuru,
    pub anahtar: String,
    pub ad: String,
    pub telefon: String,
    /// WhatsApp'a hazır metin.
    pub metin: String,
}

/// Tek oturumda teslim edilebilen, tek seferlik ücretli iş.
pub struct HizliIs {
    pub ad: &'static str,
    pub fiyat: u32,
    /// Yaklaşık teslim süresi.
    pub sure: &'static str,
    /// Müşteriye bırakılan dosya.
    pub teslim: &'static str,
    /// Satış mesajı; `{ad}` işletme adıyla değiştirilir.
    pub mesaj: &'static str,
}

/// Hızlı para getiren işler: hazır dosyayla aynı gün teslim edilir.
pub const HIZLI_ISLER: &[HizliIs] = &[
    HizliIs {
        ad: "Google profil yenileme",
        fiyat: 1_500,
        sure: "20 dk",
        teslim: "profil kontrol listesi + açıklama metni",
        mesaj: "Merhaba {ad}, Google profilinizi baştan sona kontrol edip açıklama, kategori ve foto listesini hazırlıyorum. Tek seferlik 1.500 TL, aynı gün teslim.",
    },
    HizliIs {
        ad: "QR menü / dijital kartvizit",
        fiyat: 2_500,
        sure: "30 dk",
        teslim: "telefonda açılan menü sayfası + QR",
        mesaj: "Merhaba {ad}, menünüzü QR ile telefonda açılan tek sayfaya çeviriyorum. Tek seferlik 2.500 TL, bir günde hazır.",
    },
    HizliIs {
        ad: "Yorum toplama seti",
        fiyat: 1_000,
        sure: "15 dk",
        teslim: "yorum isteme şablonları + QR",
        mesaj: "Merhaba {ad}, memnun müşteriden yorum istemek için hazır mesaj ve QR seti hazırlıyorum. Tek seferlik 1.000 TL.",
    },
    HizliIs {
        ad: "NAP / citation düzeltme",
        fiyat: 1_500,
        sure: "30 dk",
        teslim: "10 rehberde ad-adres-telefon listesi",
        mesaj: "Merhaba {ad}, adres ve telefonunuzun rehberlerde aynı görünmesi için kayıt listesi çıkarıyorum. Tek seferlik 1.500 TL.",
    },
    HizliIs {
        ad: "Tek sayfa site",
        fiyat: 4_500,
        sure: "60 dk",
        teslim: "yayına hazır tek sayfa + WhatsApp bağlantısı",
        mesaj: "Merhaba {ad}, tek sayfalık, telefonda hızlı açılan bir site hazırlıyorum. Tek seferlik 4.500 TL.",
    },
    HizliIs {
        ad: "Aylık görünürlük raporu",
        fiyat: 1_500,
        sure: "10 dk",
        teslim: "tek sayfalık durum raporu",
        mesaj: "Merhaba {ad}, Google görünürlüğünüzün tek sayfalık durum raporunu çıkarıyorum. Tek seferlik 1.500 TL.",
    },
    HizliIs {
        ad: "AI görünürlük ölçümü",
        fiyat: 2_000,
        sure: "30 dk",
        teslim: "asistanlarda görünme ölçüm formu",
        mesaj: "Merhaba {ad}, ChatGPT ve Gemini'de işletmenizin geçip geçmediğini ölçüp raporluyorum. Tek seferlik 2.000 TL.",
    },
    HizliIs {
        ad: "Rakip raporu",
        fiyat: 1_500,
        sure: "20 dk",
        teslim: "aynı bölgedeki rakiplerin puan/yorum dökümü",
        mesaj: "Merhaba {ad}, aynı bölgedeki rakiplerinizin puan ve yorum dökümünü çıkarıyorum. Tek seferlik 1.500 TL.",
    },
];

/// Ayda satılması hedeflenen hızlı iş sayısı.
pub const AYLIK_HIZLI_ADET: u32 = 10;

/// Hızlı işlerin aylık ortalama katkısı (ortalama fiyat × adet).
pub fn aylik_hizli_gelir() -> u32 {
    let toplam: u32 = HIZLI_ISLER.iter().map(|is| is.fiyat).sum();
    (toplam / HIZLI_ISLER.len() as u32) * AYLIK_HIZLI_ADET
}

/// Aboneye bu ay önerilecek ek iş; ay ve müşteri anahtarına göre döner.
pub fn ek_satis(kayit: &gelir::MusteriGelir, ay: &str) -> &'static HizliIs {
    let ay_no: usize = ay.get(5..7).and_then(|ay| ay.parse().ok()).unwrap_or(1);
    let tohum: usize = kayit.anahtar.bytes().map(usize::from).sum();
    &HIZLI_ISLER[(ay_no + tohum) % HIZLI_ISLER.len()]
}

/// 200.000 TL hedefinin günlük karşılığı.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hesap {
    /// Bugünkü aylık tekrarlayan gelir.
    pub mrr: u32,
    /// Aylık ödemesi olan müşteri sayısı.
    pub abone: usize,
    /// Hedefe kalan tutar.
    pub eksik: u32,
    /// Ortalama aylık paket tutarı.
    pub ortalama: u32,
    /// Hedef için gereken toplam abone.
    pub gereken: usize,
    /// Bugünkü aboneye göre kalan abone.
    pub eksik_abone: usize,
    /// Ayda atılabilecek mesaj sayısı.
    pub aylik_mesaj: usize,
    /// Bu tempoda ayda kazanılabilecek abone (kapanış oranıyla).
    pub aylik_yeni: usize,
    /// Hedefe kalan ay (bu tempoda).
    pub ay_gerekli: u32,
    /// Bugün atılması gereken mesaj.
    pub gunluk_mesaj: usize,
    /// Hızlı işlerden beklenen aylık ek gelir.
    pub ek_gelir: u32,
    /// Ek gelirin karşıladığı abone sayısı.
    pub ek_abone_karsiligi: usize,
}

impl Hesap {
    /// Hedefe ulaşma yüzdesi (0-100).
    pub fn yuzde(&self) -> u32 {
        ((self.mrr as u64 * 100) / HEDEF_TL as u64).min(100) as u32
    }
}

/// Hedefi günlük işe çevirir.
pub fn hesap(mrr: u32, abone: usize) -> Hesap {
    let ortalama = if abone > 0 {
        (mrr / abone as u32).max(1)
    } else {
        5_000
    };
    let gereken = ((HEDEF_TL + ortalama - 1) / ortalama) as usize;
    let eksik = HEDEF_TL.saturating_sub(mrr);
    // Hızlı işler hedefin bir kısmını karşılar; kalan aboneye bölünür.
    let ek_gelir = aylik_hizli_gelir();
    let kalan = HEDEF_TL.saturating_sub(mrr.saturating_add(ek_gelir));
    let eksik_abone = ((kalan + ortalama - 1) / ortalama) as usize;
    let aylik_mesaj = GUNLUK_MESAJ * ISI_GUNU as usize;
    let aylik_yeni = (aylik_mesaj * KAPANMA_YUZDE as usize) / 100;
    let ay_gerekli = if eksik > 0 && aylik_yeni > 0 {
        ((eksik_abone + aylik_yeni - 1) / aylik_yeni) as u32
    } else {
        0
    };
    Hesap {
        mrr,
        abone,
        eksik,
        ortalama,
        gereken,
        eksik_abone,
        aylik_mesaj,
        aylik_yeni,
        ay_gerekli,
        gunluk_mesaj: if eksik_abone > 0 { GUNLUK_MESAJ } else { 0 },
        ek_gelir,
        ek_abone_karsiligi: (ek_gelir / ortalama.max(1)) as usize,
    }
}

/// İki tarih arasındaki gün sayısı; çözülemeyen tarih `None`.
fn gun_farki(eski: &str, yeni: &str) -> Option<i64> {
    let eski = chrono::NaiveDate::parse_from_str(eski.trim(), "%Y-%m-%d").ok()?;
    let yeni = chrono::NaiveDate::parse_from_str(yeni.trim(), "%Y-%m-%d").ok()?;
    Some((yeni - eski).num_days())
}

/// Hiç yazılmamış işletmeye giden ilk mesaj (kısa, iddiasız).
fn ilk_mesaj(ad: &str, kategori: &str) -> String {
    let sektor = if kategori.trim().is_empty() {
        "işletmeniz"
    } else {
        kategori.trim()
    };
    format!(
        "Merhaba {},\n\n{} için hazırladığım görünürlük kontrol listesini paylaşabilir miyim? \
Aylık 5.000 TL'lik paketle Google profili, yorumlar ve site tarafını biz yürütüyoruz. \
İsterseniz listeyi görüp karar verin.",
        ad.trim(),
        sektor
    )
}

/// Takip zamanı gelen işletmeye giden mesaj.
fn takip_mesaji(ad: &str, gecen: i64) -> String {
    format!(
        "Merhaba {},\n\n{} gün önce gönderdiğim teklife dair bir sorunuz oldu mu? \
İsterseniz tek sayfalık özetle tekrar gönderebilirim.",
        ad.trim(),
        gecen.max(1)
    )
}

/// Bugün yazılacaklar: sıcak → takip → ilk mesaj, günlük kota kadar.
pub fn gunun_isleri(defter: &musteriler::Defter, gelir: &gelir::Gelir, gun: &str) -> Vec<Is> {
    let mut liste: Vec<Is> = Vec::new();
    for kayit in defter.kayitlar() {
        if kayit.durum.kara_listede() || kayit.telefon.trim().is_empty() {
            continue;
        }
        let gecen = kayit
            .son_gonderim
            .as_deref()
            .or(kayit.son_temas.as_deref())
            .and_then(|tarih| gun_farki(tarih, gun));
        let tur = match kayit.durum {
            musteriler::Durum::Ilgilendi => IsTuru::Sicak,
            musteriler::Durum::Gonderildi if gecen.map(|gun| gun >= TAKIP_GUN).unwrap_or(false) => {
                IsTuru::Takip
            }
            musteriler::Durum::Yeni | musteriler::Durum::MesajHazir => IsTuru::YeniMesaj,
            _ => continue,
        };
        let teklif = gelir
            .teklif(&kayit.anahtar, gun)
            .map(|teklif| teklif.metin.clone());
        let metin = match (tur, teklif) {
            (IsTuru::Takip, _) => takip_mesaji(&kayit.ad, gecen.unwrap_or(TAKIP_GUN)),
            (_, Some(metin)) => metin,
            (_, None) => ilk_mesaj(&kayit.ad, &kayit.kategori),
        };
        liste.push(Is {
            tur,
            anahtar: kayit.anahtar.clone(),
            ad: kayit.ad.clone(),
            telefon: kayit.telefon.clone(),
            metin,
        });
    }
    liste.sort_by(|a, b| {
        a.tur
            .oncelik()
            .cmp(&b.tur.oncelik())
            .then_with(|| a.ad.cmp(&b.ad))
    });
    // Günlük kota: fazlası yarına kalır, liste şişmez.
    liste.truncate(GUNLUK_MESAJ);
    liste
}

/// Otomasyon defteri: son çalışma günü ve aylık teslim edilen dosyalar.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Dosya {
    #[serde(default)]
    surum: u32,
    /// "Bugünü çalıştır" en son hangi gün çalıştı.
    #[serde(default)]
    pub son_calisma: String,
    /// Anahtar → teslim edilen ay ("2026-11").
    #[serde(default)]
    pub teslim: HashMap<String, String>,
}

impl Dosya {
    pub fn yukle() -> Self {
        let metin = fs::read_to_string(DOSYA).unwrap_or_default();
        Self::metinden(&metin)
    }

    pub fn metinden(metin: &str) -> Self {
        let mut dosya: Dosya = serde_json::from_str(metin).unwrap_or_default();
        dosya.surum = SURUM;
        dosya
    }

    pub fn kaydet(&self) {
        if let Ok(metin) = serde_json::to_string_pretty(self) {
            let _ = fs::write(DOSYA, metin);
        }
    }

    /// Bu müşterinin bu ayki dosyaları teslim edildi mi?
    pub fn teslim_edildi(&self, anahtar: &str, ay: &str) -> bool {
        self.teslim.get(anahtar).is_some_and(|son| son == ay)
    }

    pub fn teslim_et(&mut self, anahtar: &str, ay: &str) {
        self.teslim.insert(anahtar.to_string(), ay.to_string());
    }
}

/// Aylık teslimat borcu olan aboneler: (anahtar, ad).
pub fn teslim_borclari(
    gelir: &gelir::Gelir,
    defter: &musteriler::Defter,
    dosya: &Dosya,
    ay: &str,
) -> Vec<(String, String)> {
    let mut liste: Vec<(String, String)> = Vec::new();
    for kayit in gelir.kayitlar() {
        if kayit.aylik() == 0 || dosya.teslim_edildi(&kayit.anahtar, ay) {
            continue;
        }
        if let Some(musteri) = defter.kayit(&kayit.anahtar) {
            liste.push((musteri.anahtar.clone(), musteri.ad.clone()));
        }
    }
    liste.sort_by(|a, b| a.1.cmp(&b.1));
    liste
}

/// Bu ayın tahsilat özeti: (tahsil edilen aylık, bekleyen aylık).
pub fn tahsilat(gelir: &gelir::Gelir, ay: &str) -> (u32, u32) {
    let mut tahsil = 0;
    let mut bekleyen = 0;
    for kayit in gelir.kayitlar() {
        if kayit.aylik() == 0 {
            continue;
        }
        if kayit.odendi.as_deref() == Some(ay) {
            tahsil += kayit.aylik();
        } else {
            bekleyen += kayit.aylik();
        }
    }
    (tahsil, bekleyen)
}

/// Gönderilmiş ama yanıtlanmamış, takip zamanı gelen teklifler.
pub fn teklif_takibi(gelir: &gelir::Gelir, gun: &str) -> Vec<(String, String, String)> {
    let mut liste = Vec::new();
    for teklif in gelir.teklifler() {
        if !teklif.gonderildi {
            continue;
        }
        let Some(gecen) = gun_farki(&teklif.gun, gun) else {
            continue;
        };
        if gecen < TAKIP_GUN {
            continue;
        }
        liste.push((
            teklif.anahtar.clone(),
            teklif.ad.clone(),
            format!(
                "Merhaba {}, {} gün önce gönderdiğim teklife dair bir soru oldu mu? Kapsamı birlikte gözden geçirebiliriz.",
                teklif.ad.trim(),
                gecen
            ),
        ));
    }
    liste.sort_by(|a, b| a.1.cmp(&b.1));
    liste
}

/// Binlik ayraçlı tutar: 200000 → "200.000".
pub fn tl(tutar: u32) -> String {
    let rakamlar = tutar.to_string();
    let mut cikti = String::new();
    for (sira, karakter) in rakamlar.chars().enumerate() {
        if sira > 0 && (rakamlar.len() - sira) % 3 == 0 {
            cikti.push('.');
        }
        cikti.push(karakter);
    }
    cikti
}

impl LeadApp {
    /// Gelir katmanının "Otomasyon" sayfası: tek sütun, kaydırılabilir.
    pub(crate) fn otomasyon_ekrani(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_source("otomasyon_sayfa")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                self.otomasyon_icerik(ui);
            });
    }

    /// Otomasyon sayfasının içeriği.
    fn otomasyon_icerik(&mut self, ui: &mut egui::Ui) {
        let gun = bugun_anahtari();
        let ay: String = gun.chars().take(7).collect();
        let hesap = hesap(self.gelir.mrr(), self.gelir.abone_sayisi());
        let isler = gunun_isleri(&self.defter, &self.gelir, &gun);
        let borclar = teslim_borclari(&self.gelir, &self.defter, &self.otomasyon, &ay);
        let aboneler: Vec<(String, String, String, u32, String)> = self
            .gelir
            .kayitlar()
            .iter()
            .filter(|kayit| kayit.aylik() > 0)
            .map(|kayit| {
                let ek = ek_satis(kayit, &ay);
                (
                    kayit.anahtar.clone(),
                    kayit.ad.clone(),
                    ek.ad.to_string(),
                    ek.fiyat,
                    ek.mesaj.replace("{ad}", &kayit.ad),
                )
            })
            .collect();
        let hizli_hedef = isler
            .first()
            .map(|is| is.ad.clone())
            .unwrap_or_else(|| "işletmeniz".to_string());
        let (tahsil, bekleyen) = tahsilat(&self.gelir, &ay);
        let odenecek: Vec<(String, String, u32, bool)> = self
            .gelir
            .kayitlar()
            .iter()
            .filter(|kayit| kayit.aylik() > 0)
            .map(|kayit| {
                (
                    kayit.anahtar.clone(),
                    kayit.ad.clone(),
                    kayit.aylik(),
                    kayit.odendi.as_deref() == Some(ay.as_str()),
                )
            })
            .collect();
        let takip = teklif_takibi(&self.gelir, &gun);

        let mut whatsapp: Option<(String, String)> = None;
        let mut gonderildi: Option<String> = None;
        let mut uret: Option<String> = None;
        let mut kopyala = false;
        let mut bugunu_calistir = false;
        let mut odendi: Option<String> = None;

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} TL/ay", tl(HEDEF_TL)))
                    .font(baslik_font(16.0))
                    .color(METIN),
            );
            ui.label(
                egui::RichText::new(format!(
                    "· şu an {} TL · kalan {} TL",
                    tl(hesap.mrr),
                    tl(hesap.eksik)
                ))
                .size(11.0)
                .color(SOLUK),
            );
        });
        ui.add_space(4.0);
        ui.add(
            egui::ProgressBar::new(hesap.yuzde() as f32 / 100.0)
                .desired_width((ui.available_width() - 8.0).max(120.0))
                .fill(VURGU),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!(
                "%{} · {} abone / {} gerekli · {} abone daha",
                hesap.yuzde(),
                hesap.abone,
                hesap.gereken,
                hesap.eksik_abone
            ))
            .size(10.0)
            .color(SOLUK),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!(
                "Ortalama paket {} TL · günde {} mesaj · %{} kapanışla ayda ~{} abone · ayda {} hızlı iş (+{} TL) · hedefe ~{} ay",
                tl(hesap.ortalama),
                hesap.gunluk_mesaj,
                KAPANMA_YUZDE,
                hesap.aylik_yeni,
                AYLIK_HIZLI_ADET,
                tl(hesap.ek_gelir),
                hesap.ay_gerekli
            ))
            .size(10.0)
            .color(SOLUK),
        );
        ui.label(
            egui::RichText::new(
                "Hepsi yerel ve ücretsiz: dış API, abonelik ya da kredi harcanmaz. Gönderimi siz yaparsınız.",
            )
            .size(9.5)
            .color(DIM),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if birincil_buton(ui, "Bugünü çalıştır", true, None).clicked() {
                bugunu_calistir = true;
            }
            ui.add_space(6.0);
            if ikincil_buton(
                ui,
                &format!("Mesaj listesini kopyala · {}", isler.len()),
                true,
                None,
            )
            .clicked()
            {
                kopyala = true;
            }
        });
        ui.add_space(8.0);

        bolum_etiketi(ui, &format!("BUGÜN YAZILACAKLAR · {}", isler.len()));
        ui.add_space(2.0);
        if isler.is_empty() {
            ui.label(
                egui::RichText::new(
                    "Bugün sırada kimse yok: Müşteri Bulma sekmesinde tarama yapın ya da takip gününü bekleyin.",
                )
                .size(10.5)
                .color(DIM),
            );
        }
        {
            for is in &isler {
                egui::Frame::none()
                    .fill(GIRINTI)
                    .stroke(egui::Stroke::new(1.0_f32, KENAR))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{} · {}", is.tur.etiket(), is.ad))
                                        .size(11.5)
                                        .color(METIN),
                                );
                                ui.label(
                                    egui::RichText::new(if is.telefon.trim().is_empty() {
                                        "telefon yok".to_string()
                                    } else {
                                        is.telefon.clone()
                                    })
                                    .size(9.5)
                                    .color(DIM),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if hayalet_buton(ui, "Gönderdim").clicked() {
                                        gonderildi = Some(is.anahtar.clone());
                                    }
                                    if is.telefon.trim().is_empty() {
                                        if hayalet_buton(ui, "Kopyala").clicked() {
                                            ui.ctx().output_mut(|cikti| {
                                                cikti.copied_text = is.metin.clone()
                                            });
                                        }
                                    } else if ikincil_buton(ui, "WhatsApp", true, None).clicked() {
                                        whatsapp = Some((is.anahtar.clone(), is.metin.clone()));
                                    }
                                },
                            );
                        });
                    });
                ui.add_space(4.0);
            }
        }
        ui.add_space(8.0);

        bolum_etiketi(ui, &format!("BU AYIN TESLİMATI · {}", borclar.len()));
        ui.add_space(2.0);
        if borclar.is_empty() {
            ui.label(
                egui::RichText::new("Bu ayın teslimat borcu yok.")
                    .size(10.5)
                    .color(DIM),
            );
        }
        for (anahtar, ad) in &borclar {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(ad).size(11.0).color(METIN));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ikincil_buton(ui, "Dosyaları üret", true, None).clicked() {
                        uret = Some(anahtar.clone());
                    }
                });
            });
        }

        ui.add_space(10.0);
        bolum_etiketi(
            ui,
            &format!(
                "TAHSİLAT · {} · alınan {} TL · bekleyen {} TL",
                ay,
                tl(tahsil),
                tl(bekleyen)
            ),
        );
        ui.add_space(2.0);
        if odenecek.is_empty() {
            ui.label(
                egui::RichText::new("Henüz aylık ödeyen abone yok.")
                    .size(10.5)
                    .color(DIM),
            );
        }
        for (anahtar, ad, aylik, alindi) in &odenecek {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{ad} · {} TL", tl(*aylik)))
                        .size(11.0)
                        .color(METIN),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if *alindi {
                        ui.label(egui::RichText::new("ödendi ✓").size(10.0).color(YESIL));
                    } else if ikincil_buton(ui, "Ödeme alındı", true, None).clicked() {
                        odendi = Some(anahtar.clone());
                    }
                });
            });
        }

        if !takip.is_empty() {
            ui.add_space(8.0);
            bolum_etiketi(ui, &format!("TEKLİF TAKİBİ · {}", takip.len()));
            ui.add_space(2.0);
            for (anahtar, ad, metin) in &takip {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(ad).size(11.0).color(METIN));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ikincil_buton(ui, "Takip mesajı", true, None).clicked() {
                            whatsapp = Some((anahtar.clone(), metin.clone()));
                        }
                    });
                });
            }
        }

        ui.add_space(10.0);
        bolum_etiketi(
            ui,
            &format!(
                "HIZLI KAZANÇ · ayda {} iş = {} TL",
                AYLIK_HIZLI_ADET,
                tl(aylik_hizli_gelir())
            ),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(
                "Tek oturumda teslim edilir. Satıştan sonra Gelir → müşteri → Hizmet dosyaları ile çıktıyı üretin.",
            )
            .size(9.5)
            .color(DIM),
        );
        ui.add_space(2.0);
        for hizli in HIZLI_ISLER {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} · {} TL · {}",
                        hizli.ad,
                        tl(hizli.fiyat),
                        hizli.sure
                    ))
                    .size(11.0)
                    .color(METIN),
                );
                ui.label(egui::RichText::new(hizli.teslim).size(9.5).color(DIM));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if hayalet_buton(ui, "Satış mesajı").clicked() {
                        ui.ctx().output_mut(|cikti| {
                            cikti.copied_text = hizli.mesaj.replace("{ad}", &hizli_hedef)
                        });
                    }
                });
            });
        }

        if !aboneler.is_empty() {
            ui.add_space(8.0);
            bolum_etiketi(ui, "ABONELERE EK SATIŞ · bu ay");
            ui.add_space(2.0);
            for (anahtar, ad, ek_ad, fiyat, metin) in &aboneler {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{ad} → {ek_ad} · {} TL", tl(*fiyat)))
                            .size(11.0)
                            .color(METIN),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ikincil_buton(ui, "WhatsApp", true, None).clicked() {
                            whatsapp = Some((anahtar.clone(), metin.clone()));
                        }
                    });
                });
            }
        }

        // --- Değişiklikleri uygula ------------------------------------------
        if let Some((anahtar, metin)) = whatsapp {
            let telefon = self
                .defter
                .kayit(&anahtar)
                .map(|kayit| kayit.telefon.clone())
                .unwrap_or_default();
            match whatsapp_numarasi(&telefon) {
                Some(numara) => {
                    if sohbet_ac(&numara, &metin, &mut self.logs) {
                        self.logs.push(
                            "[i] WhatsApp hazır: gönderdikten sonra Gönderdim'e basın.".to_string(),
                        );
                    }
                }
                None => {
                    ui.ctx().output_mut(|cikti| cikti.copied_text = metin);
                    self.logs
                        .push("[!] Telefon okunamadı; mesaj panoya kopyalandı.".to_string());
                }
            }
        }
        if let Some(anahtar) = gonderildi {
            let gun = bugun_anahtari();
            self.defter.gonderim_kaydet(&anahtar, &gun);
            self.defter
                .durum_ayarla(&anahtar, musteriler::Durum::Gonderildi, &gun);
            self.defter.kaydet();
            self.logs
                .push("[✓] Gönderim işlendi; takip tarihi kendiliğinden kuruldu.".to_string());
        }
        if let Some(anahtar) = uret {
            match (
                self.defter.kayit(&anahtar).cloned(),
                self.gelir.kayit(&anahtar).cloned(),
            ) {
                (Some(isletme), Some(kayit)) => match teslimat::yaz(&isletme, &kayit, &gun) {
                    Ok(ciktilar) => {
                        self.otomasyon.teslim_et(&anahtar, &ay);
                        self.otomasyon.kaydet();
                        self.logs.push(format!(
                            "[✓] {}: {} dosya üretildi → {}/",
                            isletme.ad,
                            ciktilar.len(),
                            teslimat::KLASOR
                        ));
                    }
                    Err(hata) => self.logs.push(format!("[!] Dosyalar üretilemedi: {hata}")),
                },
                _ => self
                    .logs
                    .push("[!] Müşteri ya da hizmet kaydı bulunamadı.".to_string()),
            }
        }
        if kopyala {
            let metin = isler
                .iter()
                .map(|is| format!("{} — {}\n{}\n", is.ad, is.telefon, is.metin))
                .collect::<Vec<_>>()
                .join("\n");
            ui.ctx().output_mut(|cikti| cikti.copied_text = metin);
            self.logs
                .push("[✓] Bugünün mesaj listesi panoya kopyalandı.".to_string());
        }
        if bugunu_calistir {
            let mut uretilen = 0;
            let mut hata = 0;
            for (anahtar, _) in &borclar {
                let isletme = self.defter.kayit(anahtar).cloned();
                let kayit = self.gelir.kayit(anahtar).cloned();
                if let (Some(isletme), Some(kayit)) = (isletme, kayit) {
                    match teslimat::yaz(&isletme, &kayit, &gun) {
                        Ok(_) => {
                            self.otomasyon.teslim_et(anahtar, &ay);
                            uretilen += 1;
                        }
                        Err(_) => hata += 1,
                    }
                }
            }
            self.otomasyon.son_calisma = gun.clone();
            self.otomasyon.kaydet();
            self.logs.push(format!(
                "[✓] Bugün çalıştı: {uretilen} aboneye aylık dosya üretildi{}, {} kişi mesaj sırasında.",
                if hata > 0 {
                    format!(", {hata} hata")
                } else {
                    String::new()
                },
                isler.len()
            ));
        }
        if let Some(anahtar) = odendi {
            let ad = self
                .defter
                .kayit(&anahtar)
                .map(|kayit| kayit.ad.clone())
                .unwrap_or_default();
            self.gelir.odeme_isle(&anahtar, &ad, &ay);
            self.gelir.kaydet();
            self.logs
                .push(format!("[✓] {ad}: {ay} ödemesi alındı olarak işaretlendi."));
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ekle(defter: &mut musteriler::Defter, ad: &str, kategori: &str, telefon: &str) -> String {
        let anahtar = format!("osm:node/{ad}");
        defter.isle(
            &anahtar,
            ad,
            telefon,
            "Manisa",
            kategori,
            "osm",
            "yok",
            "",
            70,
            "2026-11-01",
        );
        anahtar
    }

    #[test]
    fn hedef_hesabi_dogru() {
        let hesap = hesap(0, 0);
        assert_eq!(hesap.ortalama, 5_000);
        assert_eq!(hesap.gereken, 40);
        assert_eq!(hesap.eksik, 200_000);
        assert_eq!(hesap.eksik_abone, 36);
        assert_eq!(hesap.ek_gelir, 20_000);
        assert_eq!(hesap.ek_abone_karsiligi, 4);
        assert_eq!(hesap.aylik_mesaj, 520);
        assert_eq!(hesap.aylik_yeni, 20);
        assert_eq!(hesap.ay_gerekli, 2);
        assert_eq!(hesap.gunluk_mesaj, 20);
        assert_eq!(hesap.yuzde(), 0);
    }

    #[test]
    fn hedefe_ulasinca_kota_sifirlanir() {
        let hesap = hesap(200_000, 40);
        assert_eq!(hesap.eksik, 0);
        assert_eq!(hesap.eksik_abone, 0);
        assert_eq!(hesap.ay_gerekli, 0);
        assert_eq!(hesap.gunluk_mesaj, 0);
        assert_eq!(hesap.yuzde(), 100);
    }

    #[test]
    fn ortalama_mevcut_musterilerden_hesaplanir() {
        let hesap = hesap(10_000, 2);
        assert_eq!(hesap.ortalama, 5_000);
        assert_eq!(hesap.gereken, 40);
        assert_eq!(hesap.eksik_abone, 34);
        assert_eq!(hesap.ay_gerekli, 2);
    }

    #[test]
    fn hizli_isler_aylik_ek_gelir_uretir() {
        assert_eq!(aylik_hizli_gelir(), 20_000);
        assert_eq!(HIZLI_ISLER.len(), 8);
        assert!(HIZLI_ISLER
            .iter()
            .all(|is| is.fiyat > 0 && is.sure.len() > 0));
    }

    #[test]
    fn ek_satis_onerisi_kararli_ve_metinli() {
        let kayit = gelir::MusteriGelir {
            anahtar: "osm:node/kafe".to_string(),
            ad: "Kafe".to_string(),
            paket: Some("buyume".to_string()),
            paket_fiyati: 5_000,
            ..Default::default()
        };
        let ilk = ek_satis(&kayit, "2026-11");
        let ikinci = ek_satis(&kayit, "2026-11");
        assert_eq!(ilk.ad, ikinci.ad);
        assert!(ilk.mesaj.contains("{ad}"));
        assert!(ilk.fiyat > 0);
    }

    #[test]
    fn gunun_isleri_once_sicaklari_listeler() {
        let mut defter = musteriler::Defter::default();
        let sicak = ekle(&mut defter, "Sıcak", "Kafe", "0532 123 45 67");
        defter.durum_ayarla(&sicak, musteriler::Durum::Ilgilendi, "2026-11-09");
        ekle(&mut defter, "Yeni", "Kafe", "0532 123 45 68");
        let kara = ekle(&mut defter, "Kara", "Kafe", "0532 123 45 69");
        defter.durum_ayarla(&kara, musteriler::Durum::Reddetti, "2026-11-09");
        let gelir = gelir::Gelir::metinden("");
        let isler = gunun_isleri(&defter, &gelir, "2026-11-10");
        assert_eq!(isler.len(), 2);
        assert_eq!(isler[0].tur, IsTuru::Sicak);
        assert_eq!(isler[0].ad, "Sıcak");
        assert_eq!(isler[1].tur, IsTuru::YeniMesaj);
        assert!(isler[0].metin.contains("Sıcak"));
    }

    #[test]
    fn takip_gunu_beklenmeden_yazilmaz() {
        let mut defter = musteriler::Defter::default();
        let bekleyen = ekle(&mut defter, "Bekleyen", "Kuaför", "0532 123 45 67");
        defter.durum_ayarla(&bekleyen, musteriler::Durum::Gonderildi, "2026-11-09");
        defter.gonderim_kaydet(&bekleyen, "2026-11-09");
        let gelir = gelir::Gelir::metinden("");
        assert!(gunun_isleri(&defter, &gelir, "2026-11-10").is_empty());

        let geciken = ekle(&mut defter, "Geciken", "Kuaför", "0532 123 45 68");
        defter.durum_ayarla(&geciken, musteriler::Durum::Gonderildi, "2026-11-01");
        defter.gonderim_kaydet(&geciken, "2026-11-01");
        let isler = gunun_isleri(&defter, &gelir, "2026-11-10");
        assert!(isler.iter().any(|is| is.tur == IsTuru::Takip));
    }

    #[test]
    fn telefonu_olmayan_listeye_girmez() {
        let mut defter = musteriler::Defter::default();
        ekle(&mut defter, "Telefonsuz", "Kafe", "");
        let gelir = gelir::Gelir::metinden("");
        assert!(gunun_isleri(&defter, &gelir, "2026-11-10").is_empty());
    }

    #[test]
    fn teslim_borcu_ay_bazinda_takip_edilir() {
        let mut defter = musteriler::Defter::default();
        let mut dosya = Dosya::default();
        let gelir = gelir::Gelir::metinden(
            "{\"kayitlar\":[{\"anahtar\":\"osm:node/X\",\"ad\":\"X\",\"paket\":\"buyume\",\"paket_fiyati\":5000}]}",
        );
        assert!(
            teslim_borclari(&gelir, &defter, &dosya, "2026-11").is_empty(),
            "defterde olmayan anahtar listelenmez"
        );

        ekle(&mut defter, "X", "Kafe", "0532 123 45 67");
        let borclar = teslim_borclari(&gelir, &defter, &dosya, "2026-11");
        assert_eq!(borclar.len(), 1);

        dosya.teslim_et("osm:node/X", "2026-11");
        assert!(teslim_borclari(&gelir, &defter, &dosya, "2026-11").is_empty());
        assert_eq!(teslim_borclari(&gelir, &defter, &dosya, "2026-12").len(), 1);
    }

    #[test]
    fn tahsilat_odenmis_ve_bekleyeni_ayirir() {
        let gelir = gelir::Gelir::metinden(
            "{\"kayitlar\":[{\"anahtar\":\"a\",\"ad\":\"A\",\"paket\":\"buyume\",\"paket_fiyati\":5000,\"odendi\":\"2026-11\"},{\"anahtar\":\"b\",\"ad\":\"B\",\"paket\":\"baslangic\",\"paket_fiyati\":2500}]}",
        );
        assert_eq!(tahsilat(&gelir, "2026-11"), (5_000, 2_500));
        assert_eq!(tahsilat(&gelir, "2026-12"), (0, 7_500));
    }

    #[test]
    fn teklif_takibi_eski_gonderilmis_teklifleri_listeler() {
        let mut gelir = gelir::Gelir::metinden("");
        let mut teklif = gelir::teklif_olustur(
            "osm:node/a",
            "Kafe A",
            gelir::oneri_girdi(&musteriler::Musteri::default()),
            "2026-11-01",
        );
        teklif.gonderildi = true;
        gelir.teklif_ekle(teklif);
        let liste = teklif_takibi(&gelir, "2026-11-10");
        assert_eq!(liste.len(), 1);
        assert!(liste[0].2.contains("9 gün"));
        assert!(teklif_takibi(&gelir, "2026-11-02").is_empty());
    }

    #[test]
    fn tutar_metni_binlik_ayirir() {
        assert_eq!(tl(200_000), "200.000");
        assert_eq!(tl(5_000), "5.000");
        assert_eq!(tl(950), "950");
    }
}
