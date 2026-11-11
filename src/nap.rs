//! Ad-adres-telefon (NAP) tutarlılık denetimi.
//!
//! Defterdeki her işletme için ad, adres, telefon, sektör ve kaynak alanları
//! karşılaştırılır; eksik ya da çelişkili olanlar tek listede toplanır.
//! Denetim yalnız yerel veriye bakar: ağ isteği yok, Google kotası harcanmaz.

use crate::*;
use eframe::egui;

/// Denetimde bulunan tek eksik ya da çelişki.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Eksik {
    /// Hangi alan: "Ad", "Adres", "Telefon", "Kaynak", "Kayıt".
    pub alan: &'static str,
    pub mesaj: String,
    /// `true` ise mesaj/teklif gitmeden önce düzeltilmeli.
    pub kritik: bool,
}

impl Eksik {
    fn yeni(alan: &'static str, mesaj: impl Into<String>, kritik: bool) -> Self {
        Self {
            alan,
            mesaj: mesaj.into(),
            kritik,
        }
    }
}

/// Listedeki tek satır: işletme ve eksikleri.
#[derive(Clone, Debug)]
pub struct Satir {
    pub ad: String,
    pub telefon: String,
    pub kategori: String,
    pub eksikler: Vec<Eksik>,
}

impl Satir {
    pub fn kritik_sayisi(&self) -> usize {
        self.eksikler.iter().filter(|eksik| eksik.kritik).count()
    }
}

/// Karşılaştırma anahtarı: küçük harf, noktalama atılır, boşluklar sadeleşir.
/// "Atatürk Cad. No:12" ile "atatürk cad no 12" aynı sayılır.
fn sadelestir(metin: &str) -> String {
    let mut cikti = String::with_capacity(metin.len());
    for karakter in metin.chars().flat_map(char::to_lowercase) {
        if karakter.is_alphanumeric() {
            cikti.push(karakter);
        } else if !cikti.ends_with(' ') {
            cikti.push(' ');
        }
    }
    cikti.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Telefon karşılaştırması için yalnız rakamlar.
fn rakamlar(metin: &str) -> String {
    metin.chars().filter(char::is_ascii_digit).collect()
}

/// Kaydın eksiklerini toplar. `tumu` mükerrer denetimi için defterin
/// tamamıdır; aynı telefon veya aynı ad-adres ikinci kez geçiyorsa yakalanır.
pub fn eksikleri_topla(kayit: &musteriler::Musteri, tumu: &[musteriler::Musteri]) -> Vec<Eksik> {
    let mut eksikler: Vec<Eksik> = Vec::new();

    // --- Ad ---------------------------------------------------------------
    let ad = kayit.ad.trim();
    if ad.is_empty() {
        eksikler.push(Eksik::yeni(
            "Ad",
            "İşletme adı boş; kayıt haritadaki işletmeyle eşleştirilemez.",
            true,
        ));
    } else {
        if kayit.ad != ad {
            eksikler.push(Eksik::yeni(
                "Ad",
                "Ad alanının başında ya da sonunda boşluk var.",
                false,
            ));
        }
        if ad.chars().count() < 3 {
            eksikler.push(Eksik::yeni(
                "Ad",
                "Ad çok kısa; harita kaydıyla eşleşmeyebilir.",
                false,
            ));
        }
    }

    // --- Telefon -----------------------------------------------------------
    let telefon = kayit.telefon.trim();
    if telefon.is_empty() {
        eksikler.push(Eksik::yeni(
            "Telefon",
            "Telefon yok; mesaj hazırlanamaz ve arama yapılamaz.",
            true,
        ));
    } else if !telefon_gecerli(telefon) {
        eksikler.push(Eksik::yeni(
            "Telefon",
            "Telefon okunamıyor (10 haneli değil); WhatsApp bağlantısı kurulamaz.",
            true,
        ));
    } else if !mobil_numara(telefon) {
        eksikler.push(Eksik::yeni(
            "Telefon",
            "Sabit hat: WhatsApp hesabı çıkmaz, arama gerekir.",
            false,
        ));
    }

    // --- Adres -------------------------------------------------------------
    let adres = kayit.adres.trim();
    if adres.is_empty() {
        eksikler.push(Eksik::yeni(
            "Adres",
            "Adres yok; harita kaydıyla karşılaştırma yapılamaz.",
            true,
        ));
    } else {
        if kayit.adres != adres {
            eksikler.push(Eksik::yeni(
                "Adres",
                "Adres alanının başında ya da sonunda boşluk var.",
                false,
            ));
        }
        if adres.chars().count() < 12 {
            eksikler.push(Eksik::yeni(
                "Adres",
                "Adres çok kısa; mahalle ve kapı numarası eksik olabilir.",
                false,
            ));
        } else if !adres.chars().any(|karakter| karakter.is_ascii_digit()) {
            eksikler.push(Eksik::yeni(
                "Adres",
                "Adreste sokak/No bilgisi yok; kapı numarası eklenmeli.",
                false,
            ));
        }
    }

    // --- Sektör ve kaynak --------------------------------------------------
    if kayit.kategori.trim().is_empty() {
        eksikler.push(Eksik::yeni(
            "Sektör",
            "Sektör boş; mesaj metni sektöre göre kurulamaz.",
            false,
        ));
    }
    if kayit.kaynak.trim().is_empty() {
        eksikler.push(Eksik::yeni(
            "Kaynak",
            "Kaynak bağlantısı yok; kayıt sonradan doğrulanamaz.",
            false,
        ));
    }

    // --- Mükerrer kayıt ----------------------------------------------------
    let bu_telefon = rakamlar(&kayit.telefon);
    if !bu_telefon.is_empty() {
        if let Some(diger) = tumu
            .iter()
            .find(|diger| diger.anahtar != kayit.anahtar && rakamlar(&diger.telefon) == bu_telefon)
        {
            eksikler.push(Eksik::yeni(
                "Kayıt",
                format!("Aynı telefon başka kayıtta da var: {}.", diger.ad.trim()),
                true,
            ));
        }
    }
    let bu_ad = sadelestir(&kayit.ad);
    let bu_adres = sadelestir(&kayit.adres);
    if !bu_ad.is_empty() && !bu_adres.is_empty() {
        if let Some(diger) = tumu.iter().find(|diger| {
            diger.anahtar != kayit.anahtar
                && sadelestir(&diger.ad) == bu_ad
                && sadelestir(&diger.adres) == bu_adres
        }) {
            eksikler.push(Eksik::yeni(
                "Kayıt",
                format!("Aynı ad ve adres ikinci kez kayıtlı: {}.", diger.ad.trim()),
                false,
            ));
        }
    }

    eksikler
}

/// Defterin tamamını denetler; yalnız eksik bulunan kayıtlar döner.
/// Sıra: önce kritik eksik sayısı, sonra toplam eksik, sonra ad.
pub fn denetim(kayitlar: &[musteriler::Musteri]) -> Vec<Satir> {
    let mut satirlar: Vec<Satir> = kayitlar
        .iter()
        .filter_map(|kayit| {
            let eksikler = eksikleri_topla(kayit, kayitlar);
            if eksikler.is_empty() {
                return None;
            }
            Some(Satir {
                ad: kayit.ad.trim().to_string(),
                telefon: kayit.telefon.trim().to_string(),
                kategori: kayit.kategori.trim().to_string(),
                eksikler,
            })
        })
        .collect();
    satirlar.sort_by(|a, b| {
        b.kritik_sayisi()
            .cmp(&a.kritik_sayisi())
            .then(b.eksikler.len().cmp(&a.eksikler.len()))
            .then(a.ad.cmp(&b.ad))
    });
    satirlar
}

/// Denetimi panoya kopyalanacak düz metne çevirir.
pub fn metin(satirlar: &[Satir]) -> String {
    if satirlar.is_empty() {
        return "Veri denetimi: kayıtlı işletmelerin tümü tutarlı.".to_string();
    }
    let eksik: usize = satirlar.iter().map(|satir| satir.eksikler.len()).sum();
    let kritik: usize = satirlar.iter().map(Satir::kritik_sayisi).sum();
    let mut cikti = format!(
        "VERİ DENETİMİ · {} işletme · {} eksik · {} kritik\n",
        satirlar.len(),
        eksik,
        kritik
    );
    for satir in satirlar {
        let mut baslik = if satir.ad.is_empty() {
            "(ad yok)".to_string()
        } else {
            satir.ad.clone()
        };
        if !satir.telefon.is_empty() {
            baslik.push_str(&format!(" · {}", satir.telefon));
        }
        if !satir.kategori.is_empty() {
            baslik.push_str(&format!(" · {}", satir.kategori));
        }
        cikti.push_str(&format!("\n{baslik}\n"));
        for eksik in &satir.eksikler {
            cikti.push_str(&format!(
                "  [{}] {}: {}\n",
                if eksik.kritik { "KRİTİK" } else { "uyarı" },
                eksik.alan,
                eksik.mesaj
            ));
        }
    }
    cikti
}

/// Yerelde yapılan tek düzeltme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Duzeltme {
    pub alan: &'static str,
    pub once: String,
    pub sonra: String,
}

/// Temizlik sonucunun özeti.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TemizlikRaporu {
    /// En az bir alanı düzelen kayıt sayısı.
    pub duzeltilen: usize,
    /// Yapılan toplam alan düzeltmesi.
    pub duzeltme_sayisi: usize,
    /// Tek kayda indirilen mükerrer kayıt sayısı.
    pub birlestirilen: usize,
    /// İlk birkaç düzeltmenin okunur hâli.
    pub ornekler: Vec<String>,
}

/// Fazla boşlukları teke indirir ve baş/son boşlukları atar.
fn bosluk_sadelestir(metin: &str) -> String {
    metin.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Telefonu yerel yazıma çeker: "+90 (532) 123 45 67" → "0532 123 45 67".
/// Okunamayan numara yalnız boşlukları sadeleştirilerek bırakılır.
pub fn telefon_duzelt(ham: &str) -> String {
    let kesilmis = ham.trim();
    let rakamlar: String = kesilmis.chars().filter(char::is_ascii_digit).collect();
    let yerel = rakamlar
        .strip_prefix("90")
        .or_else(|| rakamlar.strip_prefix('0'))
        .unwrap_or(&rakamlar);
    if yerel.len() != 10 {
        return bosluk_sadelestir(kesilmis);
    }
    format!(
        "0{} {} {} {}",
        &yerel[0..3],
        &yerel[3..6],
        &yerel[6..8],
        &yerel[8..10]
    )
}

/// Adres: boşluklar sadeleşir, virgül çevresi düzelir, sondaki noktalama atılır.
pub fn adres_duzelt(ham: &str) -> String {
    let sade = bosluk_sadelestir(ham).replace(" ,", ",");
    let sade = sade.replace(",,", ",");
    sade.trim_end_matches(|karakter: char| matches!(karakter, ',' | '.' | ';' | '-'))
        .trim()
        .to_string()
}

/// Sektör/kategori: boşluklar sadeleşir, ilk harf büyütülür.
pub fn kategori_duzelt(ham: &str) -> String {
    let sade = bosluk_sadelestir(ham);
    let mut karakterler = sade.chars();
    match karakterler.next() {
        Some(ilk) => ilk.to_uppercase().collect::<String>() + karakterler.as_str(),
        None => String::new(),
    }
}

/// "yok", "yok.", "-" gibi yer tutucular boş kabul edilir.
pub fn site_duzelt(ham: &str) -> String {
    let sade = bosluk_sadelestir(ham);
    match sade.to_lowercase().as_str() {
        "yok" | "yok." | "-" | "—" | "bilinmiyor" => String::new(),
        _ => sade,
    }
}

fn alan_duzelt(alan: &'static str, deger: &mut String, sonra: String, liste: &mut Vec<Duzeltme>) {
    if *deger != sonra {
        liste.push(Duzeltme {
            alan,
            once: deger.clone(),
            sonra: sonra.clone(),
        });
        *deger = sonra;
    }
}

/// Kaydı yerelde temizler: temiz kopya ve yapılan düzeltmelerin listesi.
/// Hiçbir alan uydurulmaz; yalnız yazım/biçim düzeltilir.
pub fn temizle(kayit: &musteriler::Musteri) -> (musteriler::Musteri, Vec<Duzeltme>) {
    let mut yeni = kayit.clone();
    let mut liste: Vec<Duzeltme> = Vec::new();
    let ad = bosluk_sadelestir(&yeni.ad);
    alan_duzelt("Ad", &mut yeni.ad, ad, &mut liste);
    let telefon = telefon_duzelt(&yeni.telefon);
    alan_duzelt("Telefon", &mut yeni.telefon, telefon, &mut liste);
    let adres = adres_duzelt(&yeni.adres);
    alan_duzelt("Adres", &mut yeni.adres, adres, &mut liste);
    let kategori = kategori_duzelt(&yeni.kategori);
    alan_duzelt("Sektör", &mut yeni.kategori, kategori, &mut liste);
    let site = site_duzelt(&yeni.site);
    alan_duzelt("Site", &mut yeni.site, site, &mut liste);
    let site_linki = bosluk_sadelestir(&yeni.site_linki);
    alan_duzelt(
        "Site bağlantısı",
        &mut yeni.site_linki,
        site_linki,
        &mut liste,
    );
    let kaynak = bosluk_sadelestir(&yeni.kaynak);
    alan_duzelt("Kaynak", &mut yeni.kaynak, kaynak, &mut liste);
    (yeni, liste)
}

/// İki kayıt aynı işletmeyi mi gösteriyor? Ad + adres aynıysa ya da
/// numaralar aynıysa evet.
fn ayni_isletme(a: &musteriler::Musteri, b: &musteriler::Musteri) -> bool {
    let a_ad = sadelestir(&a.ad);
    let b_ad = sadelestir(&b.ad);
    let a_adres = sadelestir(&a.adres);
    let b_adres = sadelestir(&b.adres);
    if !a_ad.is_empty() && a_ad == b_ad && !a_adres.is_empty() && a_adres == b_adres {
        return true;
    }
    let a_tel = rakamlar(&a.telefon);
    let b_tel = rakamlar(&b.telefon);
    a_tel.len() >= 10 && a_tel == b_tel
}

/// Mükerrer kaydın dolu alanlarını hedefe taşır; kara liste korunur.
fn birlestir(hedef: &mut musteriler::Musteri, diger: &musteriler::Musteri) {
    let bos = |metin: &str| metin.trim().is_empty();
    if bos(&hedef.telefon) {
        hedef.telefon = diger.telefon.clone();
    }
    if bos(&hedef.adres) {
        hedef.adres = diger.adres.clone();
    }
    if bos(&hedef.kategori) {
        hedef.kategori = diger.kategori.clone();
    }
    if bos(&hedef.kaynak) {
        hedef.kaynak = diger.kaynak.clone();
    }
    if bos(&hedef.site) {
        hedef.site = diger.site.clone();
    }
    if bos(&hedef.site_linki) {
        hedef.site_linki = diger.site_linki.clone();
    }
    if bos(&hedef.ad) {
        hedef.ad = diger.ad.clone();
    }
    hedef.puan = hedef.puan.max(diger.puan);
    if hedef.son_temas.is_none() {
        hedef.son_temas = diger.son_temas.clone();
    }
    if hedef.son_gonderim.is_none() {
        hedef.son_gonderim = diger.son_gonderim.clone();
    }
    if hedef.yayin_linki.is_none() {
        hedef.yayin_linki = diger.yayin_linki.clone();
    }
    if hedef.paket.is_none() {
        hedef.paket = diger.paket;
    }
    if hedef.teklif_tutari.is_none() {
        hedef.teklif_tutari = diger.teklif_tutari;
    }
    if hedef.teklif_tarihi.is_none() {
        hedef.teklif_tarihi = diger.teklif_tarihi.clone();
    }
    if diger.durum == musteriler::Durum::Reddetti {
        hedef.durum = musteriler::Durum::Reddetti;
    }
    for dosya in &diger.medya {
        if !hedef.medya.contains(dosya) {
            hedef.medya.push(dosya.clone());
        }
    }
    for satir in diger.notlar.lines() {
        let satir = satir.trim();
        if !satir.is_empty() && !hedef.notlar.contains(satir) {
            if !hedef.notlar.is_empty() {
                hedef.notlar.push('\n');
            }
            hedef.notlar.push_str(satir);
        }
    }
}

/// Defterin tamamını temizler ve mükerrerleri tek kayda indirir.
pub fn temizlik_uygula(
    kayitlar: &[musteriler::Musteri],
) -> (Vec<musteriler::Musteri>, TemizlikRaporu) {
    let mut rapor = TemizlikRaporu::default();
    let mut liste: Vec<musteriler::Musteri> = Vec::with_capacity(kayitlar.len());
    for kayit in kayitlar {
        let (temiz, duzeltmeler) = temizle(kayit);
        if !duzeltmeler.is_empty() {
            rapor.duzeltilen += 1;
            rapor.duzeltme_sayisi += duzeltmeler.len();
            if rapor.ornekler.len() < 3 {
                let ilk = &duzeltmeler[0];
                rapor.ornekler.push(format!(
                    "{} · {}: {} → {}",
                    temiz.ad, ilk.alan, ilk.once, ilk.sonra
                ));
            }
        }
        if let Some(eski) = liste.iter_mut().find(|eski| ayni_isletme(eski, &temiz)) {
            birlestir(eski, &temiz);
            rapor.birlestirilen += 1;
            continue;
        }
        liste.push(temiz);
    }
    (liste, rapor)
}

impl LeadApp {
    /// Denetim listesini tazeler ve paneli açar.
    pub(crate) fn denetimi_ac(&mut self) {
        self.denetim_liste = denetim(self.defter.kayitlar());
        self.denetim_panel_acik = true;
    }

    /// Tek pencerede eksik listesi: işletme başına satır, altında eksikler.
    pub(crate) fn denetim_paneli(&mut self, ctx: &egui::Context) {
        if !self.denetim_panel_acik {
            return;
        }
        let (mut kapat, mut yenile, mut kopyala, mut temizlik) = (false, false, false, false);
        let ekran = ctx.screen_rect();
        let boyut = egui::vec2(
            (ekran.width() - 220.0).clamp(560.0, 700.0),
            (ekran.height() - 220.0).clamp(380.0, 560.0),
        );
        let konum = ekran.center() - boyut * 0.5;

        egui::Area::new(egui::Id::new("denetim_ortu"))
            .order(egui::Order::Foreground)
            .fixed_pos(ekran.min)
            .show(ctx, |ui| {
                let (rect, yanit) = ui.allocate_exact_size(ekran.size(), egui::Sense::click());
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(155));
                if yanit.clicked() {
                    kapat = true;
                }
            });

        egui::Area::new(egui::Id::new("denetim_panel"))
            .order(egui::Order::Foreground)
            .fixed_pos(konum)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(KART)
                    .stroke(egui::Stroke::new(1.0_f32, KENAR_GUCLU))
                    .rounding(egui::Rounding::same(16.0))
                    .inner_margin(egui::Margin::symmetric(20.0, 16.0))
                    .show(ui, |ui| {
                        ui.set_width(boyut.x - 40.0);
                        ui.set_height(boyut.y - 32.0);
                        ui.horizontal(|ui| {
                            bolum_etiketi(ui, "VERİ DENETİMİ");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if hayalet_buton(ui, "Kapat").clicked() {
                                        kapat = true;
                                    }
                                },
                            );
                        });
                        ui.add_space(4.0);

                        let kritik: usize =
                            self.denetim_liste.iter().map(Satir::kritik_sayisi).sum();
                        let eksik: usize = self
                            .denetim_liste
                            .iter()
                            .map(|satir| satir.eksikler.len())
                            .sum();
                        ui.label(
                            egui::RichText::new(format!(
                                "{} işletme · {} eksik · {} kritik · defterde {} kayıt",
                                self.denetim_liste.len(),
                                eksik,
                                kritik,
                                self.defter.kayitlar().len()
                            ))
                            .size(10.5)
                            .color(DIM),
                        );
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ikincil_buton(ui, "Yenile", true, None).clicked() {
                                yenile = true;
                            }
                            ui.add_space(6.0);
                            if hayalet_buton(ui, "Panoya kopyala").clicked() {
                                kopyala = true;
                            }
                            ui.add_space(6.0);
                            if ikincil_buton(ui, "Eksikleri otomatik temizle", true, None)
                                .on_hover_text(
                                    "Telefon biçimi, boşluklar, adres, sektör ve mükerrer kayıtları yerelde düzeltir; eksik adres/telefon uydurmaz",
                                )
                                .clicked()
                            {
                                temizlik = true;
                            }
                        });
                        ui.add_space(8.0);

                        let liste_yuksekligi = ui.available_height().max(120.0);
                        egui::ScrollArea::vertical()
                            .id_source("denetim_listesi")
                            .auto_shrink([false, false])
                            .max_height(liste_yuksekligi)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                if self.denetim_liste.is_empty() {
                                    ui.label(
                                        egui::RichText::new(
                                            "Kayıtlı işletmelerin tümü tutarlı görünüyor: ad, adres ve telefon alanları dolu; mükerrer kayıt yok.",
                                        )
                                        .size(11.0)
                                        .color(SOLUK),
                                    );
                                    return;
                                }
                                for satir in &self.denetim_liste {
                                    let kritik = satir.kritik_sayisi();
                                    egui::Frame::none()
                                        .fill(PANEL)
                                        .stroke(egui::Stroke::new(
                                            1.0_f32,
                                            if kritik > 0 { ton(KIRMIZI, 0.55) } else { KENAR },
                                        ))
                                        .rounding(egui::Rounding::same(10.0))
                                        .inner_margin(egui::Margin::symmetric(12.0, 9.0))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                let ad = if satir.ad.is_empty() {
                                                    "(ad yok)".to_string()
                                                } else {
                                                    satir.ad.clone()
                                                };
                                                ui.label(
                                                    egui::RichText::new(ad)
                                                        .font(baslik_font(11.5))
                                                        .color(METIN),
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if kritik > 0 {
                                                            pill(
                                                                ui,
                                                                &format!("{kritik} kritik"),
                                                                KIRMIZI,
                                                            );
                                                        }
                                                        pill(
                                                            ui,
                                                            &format!("{} eksik", satir.eksikler.len()),
                                                            if kritik > 0 { TURUNCU } else { DIM },
                                                        );
                                                    },
                                                );
                                            });
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} · {}",
                                                    if satir.telefon.is_empty() {
                                                        "telefon yok"
                                                    } else {
                                                        satir.telefon.as_str()
                                                    },
                                                    if satir.kategori.is_empty() {
                                                        "sektör yok"
                                                    } else {
                                                        satir.kategori.as_str()
                                                    }
                                                ))
                                                .size(9.5)
                                                .color(DIM),
                                            );
                                            ui.add_space(4.0);
                                            for eksik in &satir.eksikler {
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.spacing_mut().item_spacing.x = 5.0;
                                                    pill(
                                                        ui,
                                                        eksik.alan,
                                                        if eksik.kritik { KIRMIZI } else { TURUNCU },
                                                    );
                                                    ui.label(
                                                        egui::RichText::new(&eksik.mesaj)
                                                            .size(10.0)
                                                            .color(if eksik.kritik { METIN } else { SOLUK }),
                                                    );
                                                });
                                            }
                                        });
                                    ui.add_space(6.0);
                                }
                            });
                    });
            });

        if yenile {
            self.denetim_liste = denetim(self.defter.kayitlar());
            self.logs.push(format!(
                "[i] Veri denetimi: {} kayıtta eksik bulundu.",
                self.denetim_liste.len()
            ));
        }
        if kopyala {
            ctx.output_mut(|cikti| cikti.copied_text = metin(&self.denetim_liste));
            self.logs
                .push("[✓] Veri denetimi panoya kopyalandı.".to_string());
        }
        if temizlik {
            let (liste, rapor) = temizlik_uygula(self.defter.kayitlar());
            self.defter.kayitlari_degistir(liste);
            self.defter.kaydet();
            self.denetim_liste = denetim(self.defter.kayitlar());
            self.logs.push(format!(
                "[✓] Otomatik temizlik: {} kayıtta {} alan düzeltildi · {} mükerrer birleştirildi · kalan {} kayıt eksikli.",
                rapor.duzeltilen,
                rapor.duzeltme_sayisi,
                rapor.birlestirilen,
                self.denetim_liste.len()
            ));
            for ornek in &rapor.ornekler {
                self.logs.push(format!("[i] {ornek}"));
            }
        }
        if kapat {
            self.denetim_panel_acik = false;
        }
    }
}

#[cfg(test)]
mod temizlik_testleri {
    use super::*;

    fn kayit(ad: &str, telefon: &str, adres: &str) -> musteriler::Musteri {
        musteriler::Musteri {
            ad: ad.to_string(),
            telefon: telefon.to_string(),
            adres: adres.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn telefon_yerel_yazima_cevrilir() {
        assert_eq!(telefon_duzelt("+90 (532) 123 45 67"), "0532 123 45 67");
        assert_eq!(telefon_duzelt("0532 123 45 67"), "0532 123 45 67");
        assert_eq!(telefon_duzelt("5321234567"), "0532 123 45 67");
        // Okunamayan numara olduğu gibi (yalnız boşlukları sadeleşerek) kalır.
        assert_eq!(telefon_duzelt(" 0212 444 "), "0212 444");
    }

    #[test]
    fn bosluk_ve_yer_tutucu_temizlenir() {
        let mut ham = kayit("  Kantin  ", " 0532  123 45 67 ", "  Atatürk Cad. No:5 , ");
        ham.site = "yok".to_string();
        let (temiz, duzeltmeler) = temizle(&ham);
        assert_eq!(temiz.ad, "Kantin");
        assert_eq!(temiz.telefon, "0532 123 45 67");
        assert_eq!(temiz.adres, "Atatürk Cad. No:5");
        assert!(temiz.site.is_empty());
        assert!(!duzeltmeler.is_empty());
    }

    #[test]
    fn mukerrer_kayit_birlesir_ve_kara_liste_korunur() {
        let bir = kayit("Kantin", "", "Atatürk Cad. No:5");
        let mut iki = kayit(" kantin ", "+90 532 123 45 67", "Atatürk Cad No:5");
        iki.durum = musteriler::Durum::Reddetti;
        iki.puan = 80;
        let (liste, rapor) = temizlik_uygula(&[bir, iki]);
        assert_eq!(liste.len(), 1);
        assert_eq!(rapor.birlestirilen, 1);
        assert_eq!(liste[0].telefon, "0532 123 45 67");
        assert_eq!(liste[0].puan, 80);
        assert_eq!(liste[0].durum, musteriler::Durum::Reddetti);
    }

    #[test]
    fn temiz_kayit_degismez() {
        let ham = kayit("Kantin", "0532 123 45 67", "Atatürk Cad. No:5");
        let (temiz, duzeltmeler) = temizle(&ham);
        assert!(duzeltmeler.is_empty());
        assert_eq!(temiz.ad, ham.ad);
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn kayit(ad: &str, telefon: &str, adres: &str) -> musteriler::Musteri {
        musteriler::Musteri {
            anahtar: format!("{ad}|{telefon}"),
            ad: ad.to_string(),
            telefon: telefon.to_string(),
            adres: adres.to_string(),
            kategori: "Kafe".to_string(),
            kaynak: "https://www.openstreetmap.org/node/7".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn tam_kayitta_eksik_cikmaz() {
        let tek = kayit(
            "Deneme Kafe",
            "0532 123 45 67",
            "Atatürk Cad. No:12, Kadıköy",
        );
        assert!(eksikleri_topla(&tek, std::slice::from_ref(&tek)).is_empty());
    }

    #[test]
    fn telefon_ve_adres_eksikleri_kritik() {
        let bos = kayit("Deneme Kafe", "", "");
        let eksikler = eksikleri_topla(&bos, std::slice::from_ref(&bos));
        assert!(eksikler
            .iter()
            .any(|eksik| eksik.alan == "Telefon" && eksik.kritik));
        assert!(eksikler
            .iter()
            .any(|eksik| eksik.alan == "Adres" && eksik.kritik));
    }

    #[test]
    fn bozuk_telefon_yakalanir() {
        let kayitli = kayit("Deneme Kafe", "12345", "Atatürk Cad. No:12, Kadıköy");
        let eksikler = eksikleri_topla(&kayitli, std::slice::from_ref(&kayitli));
        let telefon = eksikler
            .iter()
            .find(|eksik| eksik.alan == "Telefon")
            .expect("telefon eksik bildirilmeli");
        assert!(telefon.kritik);
        assert!(telefon.mesaj.contains("10 haneli"));
    }

    #[test]
    fn sabit_hat_kritik_degil() {
        let kayitli = kayit(
            "Deneme Kafe",
            "0212 444 62 62",
            "Atatürk Cad. No:12, Kadıköy",
        );
        let eksikler = eksikleri_topla(&kayitli, std::slice::from_ref(&kayitli));
        let telefon = eksikler
            .iter()
            .find(|eksik| eksik.alan == "Telefon")
            .expect("sabit hat uyarısı bekleniyor");
        assert!(!telefon.kritik);
        assert!(telefon.mesaj.contains("Sabit hat"));
    }

    #[test]
    fn ayni_telefon_mukerrer_sayilir() {
        let bir = kayit("Bir Kafe", "0532 123 45 67", "Atatürk Cad. No:12, Kadıköy");
        let iki = kayit("İki Kafe", "0532 123 45 67", "Moda Cad. No:8, Kadıköy");
        let hepsi = vec![bir.clone(), iki];
        let eksikler = eksikleri_topla(&bir, &hepsi);
        assert!(eksikler
            .iter()
            .any(|eksik| eksik.alan == "Kayıt" && eksik.kritik && eksik.mesaj.contains("telefon")));
    }

    #[test]
    fn ayni_ad_adres_mukerrer_sayilir() {
        let bir = kayit("Bir Kafe", "0532 123 45 67", "Atatürk Cad. No:12, Kadıköy");
        let iki = kayit("bir kafe", "0532 999 88 77", "atatürk cad no 12 kadıköy");
        let hepsi = vec![bir.clone(), iki];
        let eksikler = eksikleri_topla(&bir, &hepsi);
        assert!(eksikler.iter().any(|eksik| eksik.alan == "Kayıt"));
    }

    #[test]
    fn kritik_eksikler_listede_one_cikar() {
        let tek = kayit(
            "Deneme Kafe",
            "0532 123 45 67",
            "Atatürk Cad. No:12, Kadıköy",
        );
        let eksik_telefon = kayit("İki Kafe", "", "Moda Cad. No:8, Kadıköy");
        let eksik_adres = kayit("Üç Kafe", "0532 444 55 66", "");
        let liste = denetim(&[tek, eksik_telefon, eksik_adres]);
        assert_eq!(liste.len(), 2);
        assert!(liste[0].kritik_sayisi() >= 1);
    }

    #[test]
    fn metin_ozeti_eksikleri_yazar() {
        let bos = kayit("Deneme Kafe", "", "");
        let liste = denetim(&[bos]);
        let ozet = metin(&liste);
        assert!(ozet.contains("VERİ DENETİMİ"));
        assert!(ozet.contains("KRİTİK"));
        assert!(ozet.contains("Telefon"));
    }

    #[test]
    fn bos_defter_temiz_rapor_verir() {
        assert!(denetim(&[]).is_empty());
        assert!(metin(&[]).contains("tutarlı"));
    }
}
