//! Beş katmanlı çalışma akışının arayüzü.
//!
//! `main.rs` yalnızca kabuğu (üst şerit, Katman 1 listesi) çizer; bu modül
//! katman çubuğunu, kotayı, radarı ve 2–5. katmanların ekranlarını ekler.
//! Katmanlar sırayla iş bitirir:
//!
//! 1. Müşteri Bulma  → kimlere yazılacak, kota ne durumda
//! 2. Medya Hazırlığı → stok seçimi, yükleme ve doğrulama
//! 3. Demo Önizleme   → sayfa uygulamanın içinde, canlı düzenlenir
//! 4. Domain & Yayın  → alan adı, temiz derleme, canlıya alma
//! WhatsApp           → müşterinin yanıtı, niyet, satış
//!
//! Hiçbir katman dış tarayıcıya yönlendirmez.

use crate::*;
use eframe::egui;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Katman çubuğu ve kota şeridi
// ---------------------------------------------------------------------------

impl LeadApp {
    /// Üst şerit: beş katman + WhatsApp freni + köprü durumu.
    pub(crate) fn katman_bari(&mut self, ctx: &egui::Context) {
        let (_, _okunmamis, sicak) = self.gelen.sayilar();

        egui::TopBottomPanel::top("katman_bari")
            .exact_height(46.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(22.0, 0.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let (kutu, _) =
                        ui.allocate_exact_size(egui::vec2(62.0, 22.0), egui::Sense::hover());
                    ui.painter().rect_filled(
                        kutu,
                        egui::Rounding::same(6.0),
                        egui::Color32::from_rgb(11, 19, 31),
                    );
                    ui.painter().text(
                        kutu.center(),
                        egui::Align2::CENTER_CENTER,
                        "NEXUS",
                        baslik_font(9.0),
                        egui::Color32::from_rgb(229, 169, 59),
                    );
                    ui.add_space(10.0);

                    let mut secilen: Option<Katman> = None;
                    for katman in Katman::HEPSI {
                        let secili = self.katman == katman;
                        // Sekme başlığı sade: rakam/rozet yok.
                        let metin = katman.kisa().to_string();
                        let buton = egui::Button::new(egui::RichText::new(metin).size(10.5).color(
                            if secili {
                                KOYU_YAZI
                            } else if katman == Katman::WhatsApp && sicak > 0 {
                                KIRMIZI
                            } else {
                                SOLUK
                            },
                        ))
                        .fill(if secili {
                            VURGU
                        } else {
                            egui::Color32::TRANSPARENT
                        })
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            if secili { VURGU } else { KENAR },
                        ))
                        .rounding(egui::Rounding::same(99.0))
                        .min_size(egui::vec2(0.0, 26.0));
                        if ui.add(buton).clicked() && !secili {
                            secilen = Some(katman);
                        }
                    }
                    if let Some(katman) = secilen {
                        self.katman = katman;
                        self.logs
                            .push(format!("[i] {} · çalışma alanı değişti", katman.etiket()));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (renk, metin) = if self.kopru.calisiyor {
                            (YESIL, format!("köprü :{}", self.kopru.port))
                        } else {
                            (DIM, "köprü kapalı".to_string())
                        };
                        let (nokta, _) =
                            ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
                        ui.painter().circle_filled(nokta.center(), 3.0, renk);
                        ui.label(egui::RichText::new(metin).size(9.5).color(renk));
                        // WhatsApp freni kota şeridinde tek yerde gösterilir.
                    });
                });
            });
    }

    /// "API & Kota Yönetim" penceresi: anahtarı güvenli girin, test edin,
    /// istek sınırlarını ayarlayın ve canlı kota durumunu izleyin.
    pub(crate) fn api_paneli(&mut self, ctx: &egui::Context) {
        if !self.api_panel {
            return;
        }
        let ozet = google::ozet(self.api_gunluk, self.api_aylik);
        let kasa = anahtar::Kasa::yukle();
        let mut acik = true;
        egui::Window::new("API & Kota Yönetim")
            .open(&mut acik)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(560.0)
            .frame(
                egui::Frame::none()
                    .fill(KART)
                    .stroke(egui::Stroke::new(1.0_f32, KENAR_GUCLU))
                    .rounding(egui::Rounding::same(16.0))
                    .inner_margin(egui::Margin::symmetric(22.0, 18.0)),
            )
            .show(ctx, |ui| {
                ui.set_width(520.0);
                ui.label(egui::RichText::new("GOOGLE MAPS PLATFORM").size(9.0).color(DIM));
                ui.label(
                    egui::RichText::new("Places API (New) · Geocoding")
                        .size(13.0)
                        .color(METIN),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(kasa.kaynak_metni())
                        .size(10.0)
                        .color(if kasa.anahtar().is_some() { YESIL } else { TURUNCU }),
                );
                ui.add_space(12.0);
                ui.label(egui::RichText::new("Google Cloud API anahtarı").size(10.0).color(DIM));
                ui.add(
                    egui::TextEdit::singleline(&mut self.api_girdi)
                        .password(true)
                        .hint_text("AIza… (39 karakter)")
                        .desired_width(f32::INFINITY),
                );
                ui.label(
                    egui::RichText::new(
                        "Anahtar AES-256-GCM ile şifrelenip api_anahtari.enc dosyasına yazılır; düz metin saklanmaz.",
                    )
                    .size(9.0)
                    .color(DIM),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add(api_dugmesi("KAYDET")).clicked() {
                        let girdi = self.api_girdi.trim().to_string();
                        match anahtar::Kasa::kaydet(&girdi) {
                            Ok(()) => {
                                // Sıcak yeniden yükleme: .exe yeniden başlamaz.
                                // Harita döşemesi, bulut sayaçları ve oturum
                                // önbelleği temizlenir; yeni anahtar anında devreye girer.
                                google::bulut_temizle();
                                self.api_girdi.clear();
                                let proje = ayar_oku().google_proje;
                                if !proje.is_empty() && self.api_bulut.is_none() {
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    let anahtar = girdi.clone();
                                    std::thread::spawn(move || {
                                        let _ = tx.send(google::bulut_senkron(&anahtar, &proje));
                                    });
                                    self.api_bulut = Some(rx);
                                    self.api_mesaj = "✓ Anahtar kaydedildi; bağlantı sınanıyor ve bulut sayaçları yenileniyor…".to_string();
                                } else if self.api_test.is_none() {
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    let anahtar = girdi.clone();
                                    std::thread::spawn(move || {
                                        let _ = tx.send(google::anahtar_testi(&anahtar));
                                    });
                                    self.api_test = Some(rx);
                                    self.api_mesaj = "✓ Anahtar kaydedildi; bağlantı sınanıyor…".to_string();
                                } else {
                                    self.api_mesaj = "✓ Anahtar kaydedildi; harita yeni anahtarla yükleniyor.".to_string();
                                }
                            }
                            Err(hata) => self.api_mesaj = format!("[!] {hata}"),
                        }
                    }
                    let test_aktif = !self.api_girdi.trim().is_empty() && self.api_test.is_none();
                    if ui
                        .add_enabled(
                            test_aktif,
                            egui::Button::new(egui::RichText::new("TEST ET").size(9.5)),
                        )
                        .clicked()
                    {
                        let anahtar = self.api_girdi.trim().to_string();
                        let (tx, rx) = std::sync::mpsc::channel();
                        std::thread::spawn(move || {
                            let sonuc = google::anahtar_testi(&anahtar);
                            let _ = tx.send(sonuc);
                        });
                        self.api_test = Some(rx);
                        self.api_mesaj = "Anahtar sınanıyor…".to_string();
                    }
                    if kasa.anahtar().is_some()
                        && ui
                            .add(egui::Button::new(
                                egui::RichText::new("ANAHTARI SİL").size(9.5),
                            ))
                            .clicked()
                    {
                        match anahtar::Kasa::sil() {
                            Ok(()) => {
                                google::bulut_temizle();
                                self.api_mesaj = "Anahtar silindi.".to_string();
                            }
                            Err(hata) => self.api_mesaj = format!("[!] {hata}"),
                        }
                    }
                });
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("Google Cloud proje kimliği (gerçek kullanım)")
                        .size(10.0)
                        .color(DIM),
                );
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.api_proje)
                            .hint_text("nexus-enterprise-123456")
                            .desired_width(250.0),
                    );
                    if ui.add(api_dugmesi("PROJEYİ KAYDET")).clicked() {
                        let mut ayar = ayar_oku();
                        ayar.google_proje = self.api_proje.trim().to_string();
                        ayar_yaz(&ayar);
                        self.api_mesaj = if ayar.google_proje.is_empty() {
                            "Proje kimliği silindi; bulut senkronu kapalı.".to_string()
                        } else {
                            format!("✓ Proje kaydedildi: {}", ayar.google_proje)
                        };
                    }
                    let hazir = !self.api_proje.trim().is_empty()
                        && kasa.anahtar().is_some()
                        && self.api_bulut.is_none();
                    if ui
                        .add_enabled(
                            hazir,
                            egui::Button::new(egui::RichText::new("ŞİMDİ SENKRONLA").size(9.0)),
                        )
                        .clicked()
                    {
                        let proje = self.api_proje.trim().to_string();
                        let anahtar = kasa.anahtar().unwrap_or("").to_string();
                        let (tx, rx) = std::sync::mpsc::channel();
                        std::thread::spawn(move || {
                            let _ = tx.send(google::bulut_senkron(&anahtar, &proje));
                        });
                        self.api_bulut = Some(rx);
                        self.api_mesaj = "Google Cloud kullanımı okunuyor…".to_string();
                    }
                });
                let bulut_metni = match (ozet.bulut_gunluk, ozet.bulut_aylik, &ozet.bulut_zaman) {
                    (Some(gun), Some(ay), Some(zaman)) => format!(
                        "Bulut: bugün {gun} · bu ay {ay} istek · Places {} · Geocoding {} · son senkron {zaman} · 60 sn'de bir",
                        ozet.bulut_places.unwrap_or(0),
                        ozet.bulut_geocode.unwrap_or(0)
                    ),
                    _ => "Bulut verisi bekleniyor: proje kimliği + API anahtarı gerekir (60 sn'de bir senkron)."
                        .to_string(),
                };
                ui.label(egui::RichText::new(bulut_metni).size(9.0).color(DIM));
                if self.api_bulut.is_some() {
                    ui.label(
                        egui::RichText::new("Google Cloud kullanımı senkronize ediliyor…")
                            .size(9.0)
                            .color(TURUNCU),
                    );
                }
                if let Some(hata) = &ozet.bulut_hata {
                    ui.label(
                        egui::RichText::new(format!("Bulut hatası: {hata}"))
                            .size(9.0)
                            .color(TURUNCU),
                    );
                }
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.add(
                    egui::ProgressBar::new(ozet.gun_yuzde as f32)
                        .text(format!(
                            "Günlük {} / {} · kalan {}",
                            ozet.gunluk, ozet.gunluk_limit, ozet.kalan_gun
                        ))
                        .desired_width(508.0),
                );
                ui.add(
                    egui::ProgressBar::new(ozet.ay_yuzde as f32)
                        .text(format!(
                            "Aylık {} / {} · kalan {}",
                            ozet.aylik, ozet.aylik_limit, ozet.kalan_ay
                        ))
                        .desired_width(508.0),
                );
                ui.add(
                    egui::ProgressBar::new(ozet.ucretsiz_yuzde as f32)
                        .text(format!(
                            "Aylık ücretsiz kota %{:.2} ({} / {} istek)",
                            ozet.ucretsiz_yuzde * 100.0,
                            ozet.bulut_aylik.unwrap_or(ozet.aylik),
                            google::AYLIK_UCRETSIZ_ISTEK
                        ))
                        .desired_width(508.0),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Günlük {} / {} · Aylık {} / {} · Canlı İstek {} · oturum {} · toplam {} · güvenlik eşiği {}",
                        ozet.gunluk,
                        ozet.gunluk_limit,
                        ozet.aylik,
                        ozet.aylik_limit,
                        ozet.ucus,
                        ozet.oturum,
                        ozet.toplam,
                        ozet.guvenlik_limiti
                    ))
                    .size(9.5)
                    .color(DIM),
                );
                if ozet.manuel_gerekli {
                    ui.label(
                        egui::RichText::new(format!(
                            "DİKKAT: Günlük Güvenlik Sınırına Ulaşıldı ({}/{}) - Kalan Kendi Limiti: {} İstek",
                            ozet.gunluk,
                            ozet.gunluk_limit,
                            ozet.gunluk_limit.saturating_sub(ozet.gunluk)
                        ))
                        .size(10.0)
                        .color(TURUNCU),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "Otomatik tarama kilitli · kalan istekler yalnızca onayla harcanır · yeni kota: {} (UTC 00:00)",
                            geri_sayim(ozet.sifirlanma)
                        ))
                        .size(9.0)
                        .color(DIM),
                    );
                    if ui
                        .add(api_dugmesi(&format!(
                            "KALAN {} İSTEĞİ KULLAN (ONAY)",
                            ozet.gunluk_limit.saturating_sub(ozet.gunluk)
                        )))
                        .clicked()
                    {
                        self.kota_onay = true;
                        self.api_mesaj =
                            "Onay verildi: bir sonraki tarama kalan istekleri kullanabilir."
                                .to_string();
                    }
                } else if ozet.seviye != google::Seviye::Normal {
                    ui.label(
                        egui::RichText::new(
                            "⚠ Maliyet uyarısı: istek sınırına yaklaşıldı; otomatik tarama kısa süre sonra duracak.",
                        )
                        .size(9.5)
                        .color(KIRMIZI),
                    );
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Günlük").size(9.5).color(DIM));
                    ui.add(egui::DragValue::new(&mut self.api_gunluk).clamp_range(1..=500));
                    ui.label(egui::RichText::new("Aylık").size(9.5).color(DIM));
                    ui.add(egui::DragValue::new(&mut self.api_aylik).clamp_range(10..=10_000));
                    ui.label(egui::RichText::new("Sayfa").size(9.5).color(DIM));
                    ui.add(egui::DragValue::new(&mut self.api_sayfa).clamp_range(1..=3));
                    if ui.button("Sınırları kaydet").clicked() {
                        let mut ayar = ayar_oku();
                        ayar.google_gunluk_limit = self.api_gunluk;
                        ayar.google_aylik_limit = self.api_aylik;
                        ayar.google_sayfa = self.api_sayfa;
                        ayar_yaz(&ayar);
                        self.api_mesaj = "✓ İstek sınırları güncellendi.".to_string();
                    }
                });
                if !self.api_mesaj.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(&self.api_mesaj)
                            .size(9.5)
                            .color(if self.api_mesaj.starts_with("[!]") { KIRMIZI } else { YESIL }),
                    );
                }
            });
        if !acik {
            self.api_panel = false;
        }
    }

    /// Arka plandaki anahtar testinin sonucunu bir kez okur.
    pub(crate) fn api_test_poll(&mut self) {
        let Some(rx) = self.api_test.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(sonuc) => {
                self.api_mesaj = match sonuc {
                    Ok(mesaj) => format!("✓ {mesaj}"),
                    Err(hata) => format!("[!] {hata}"),
                };
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                self.api_test = Some(rx);
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.api_mesaj = "[!] Anahtar testi yarıda kaldı.".to_string();
            }
        }
    }

    /// Arka plandaki "Şimdi senkronla" isteğinin sonucunu bir kez okur.
    pub(crate) fn api_senkron_poll(&mut self) {
        let Some(rx) = self.api_bulut.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(ozet)) => {
                google::bulut_yaz(&ozet);
                self.api_mesaj = format!(
                    "✓ Google Cloud: bugün {} · bu ay {} istek ({}).",
                    ozet.gunluk, ozet.aylik, ozet.zaman
                );
            }
            Ok(Err(hata)) => {
                google::bulut_hata_yaz(&hata);
                self.api_mesaj = format!("[!] {hata}");
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.api_bulut = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.api_mesaj = "[!] Bulut senkronu yarıda kaldı.".to_string();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Katman yönlendirmesi
// ---------------------------------------------------------------------------

impl LeadApp {
    /// Katman 2–5'in ekranı. Katman 1 `main.rs` içinde çizilir.
    pub(crate) fn katman_ekrani(&mut self, ctx: &egui::Context, gonderen: &str) {
        match self.katman {
            Katman::MusteriBulma => {}
            Katman::WhatsApp => self.gelen_ekrani(ctx),
            Katman::Pinleme => self.pin_ekrani(ctx),
            Katman::Bolgesel => self.bolgesel_ekrani(ctx),
            Katman::Otoyol => self.otoyol_pin_ekrani(ctx),
        }
        let _ = gonderen;
    }

    /// Arşivden tek dosyayı kaldırır: dosya `.gecici/red/` altına taşınır
    /// (geri alınabilir), onay kaydından ve demo seçimlerinden düşer.
    /// Arşiv her karede yeniden tarandığı için liste ve sayaç anında güncellenir.
    fn medya_dosya_sil(&mut self, yol: &str) {
        let goreli = yol.replace('\\', "/");
        match medya_arsivden_kaldir(Path::new("."), &goreli, Path::new(".gecici/red")) {
            Ok(hedef) => {
                medya_islem::kaydet(
                    Path::new("."),
                    "SİL",
                    &medya_islem::tam_yol(Path::new(&goreli)),
                    &hedef.to_string_lossy().replace('\\', "/"),
                );
                let onay_dustu = medya_onay::dosya_unut(Path::new("."), &goreli);
                let mut secimden_dustu = false;
                for lead in self.leads.iter_mut() {
                    if lead.medya.iter().any(|kalan| kalan == &goreli) {
                        lead.medya.retain(|kalan| kalan != &goreli);
                        let anahtar =
                            musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum);
                        self.defter.medya_ayarla(&anahtar, lead.medya.clone());
                        secimden_dustu = true;
                    }
                }
                if secimden_dustu {
                    self.defter.kaydet();
                }
                let on = format!("{goreli}|");
                self.medya_onizleme
                    .retain(|anahtar, _| anahtar != &goreli && !anahtar.starts_with(&on));
                if self.medya_buyuk_onizleme.as_deref() == Some(goreli.as_str()) {
                    self.medya_buyuk_onizleme = None;
                }
                // Bir sonraki karede arşiv baştan taranır: thumbnail önbelleği
                // ve imza sıfırlanır, sayaç düşer.
                self.medya_arsiv_imza = None;
                let konan = hedef
                    .file_name()
                    .map(|ad| ad.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.medya_mesaj = format!(
                    "[✓] {} arşivden kaldırıldı · .gecici/red/{konan}",
                    dosya_adi(&goreli)
                );
                if onay_dustu {
                    self.medya_mesaj
                        .push_str(" · onay kaydından düşüldü, kota açıldı");
                }
                if secimden_dustu {
                    self.medya_mesaj.push_str(" · demo seçiminden çıkarıldı");
                }
                self.logs.push(self.medya_mesaj.clone());
            }
            Err(hata) => {
                self.medya_mesaj = format!("[!] {hata}");
                self.logs.push(self.medya_mesaj.clone());
            }
        }
    }

    /// Son üretilen yerel AI karesini seçili işletmenin demo medyasına ekler.
    /// Dosya zaten arşivdedir (`media/stock/<slug>/`); bu düğme demoya bağlar.
    fn uretilen_fotografi_ekle(&mut self, yol: &str) {
        let goreli = yol.replace('\\', "/");
        let Some(idx) = self.medya_lead else {
            self.medya_mesaj =
                "[!] Önce 2 Medya'da bir işletme seçin; görsel o işletmenin demosuna eklenir."
                    .to_string();
            return;
        };
        let Some(lead) = self.leads.get(idx) else {
            return;
        };
        let anahtar = musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum);
        let ad = dosya_adi(&goreli).to_string();
        if lead.medya.iter().any(|kalan| kalan == &goreli) {
            self.medya_mesaj = format!("[i] {ad} zaten bu işletmenin demosunda.");
            return;
        }
        let mut medya = lead.medya.clone();
        medya.push(goreli);
        if let Some(lead) = self.leads.get_mut(idx) {
            lead.medya = medya.clone();
        }
        self.defter.medya_ayarla(&anahtar, medya.clone());
        self.defter.kaydet();
        self.medya_arsiv_turu = MedyaTuru::Foto;
        self.medya_arsiv_imza = None;
        self.medya_mesaj = format!(
            "[✓] {ad} arşivden demoya eklendi · demo medyası: {}",
            medya_ozeti(&medya)
        );
        self.logs.push(self.medya_mesaj.clone());
    }

    /// Gömülü tarayıcı açıkken orta panelin yerini tutan çerçeve. Gerçek
    /// WebView2 bu dikdörtgenin üzerine native alt pencere olarak oturur.
    #[cfg(target_os = "windows")]
    fn medya_tarayici_paneli(&mut self, ui: &mut egui::Ui, slug: &str) {
        let Some((ad, url)) = self.medya_tarayici_hedef.clone() else {
            return;
        };
        ui.horizontal(|ui| {
            if ikincil_buton(ui, "← Medyaya dön", true, Some(egui::vec2(116.0, 28.0))).clicked()
            {
                self.medya_tarayici_kapat();
            }
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&ad)
                    .font(baslik_font(12.0))
                    .color(METIN),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ikincil_buton(ui, "Dış tarayıcıda aç", true, Some(egui::vec2(126.0, 26.0)))
                    .clicked()
                {
                    let _ = crate::tarayicida_ac(&url);
                }
                pill(ui, "ÇEREZLER KALICI", YESIL);
            });
        });
        ui.label(
            egui::RichText::new(format!("{url} · indirme: media/stock/{slug}/"))
                .font(kod_font(9.0))
                .color(DIM),
        );
        ui.add_space(4.0);
        let rect = ui.available_rect_before_wrap();
        self.medya_tarayici_rect = Some(rect);
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(10.0),
            egui::Color32::from_rgb(8, 9, 12),
        );
        ui.allocate_ui_at_rect(rect.shrink(20.0), |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((rect.height() * 0.28).max(18.0));
                ui.label(
                    egui::RichText::new("WebView2 gömülü oturum")
                        .font(baslik_font(16.0))
                        .color(egui::Color32::from_rgb(235, 238, 246)),
                );
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new(
                        "Kendi mailinizle giriş yapın. Çerezler sonraki açılışta korunur.",
                    )
                    .size(10.5)
                    .color(egui::Color32::from_rgb(160, 168, 184)),
                );
                ui.add_space(10.0);
                ui.add(egui::Spinner::new().size(18.0).color(VURGU));
                if !self.medya_tarayici_mesaj.is_empty() {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(&self.medya_tarayici_mesaj)
                            .size(9.5)
                            .color(if self.medya_tarayici_mesaj.starts_with("[!]") {
                                KIRMIZI
                            } else {
                                YESIL
                            }),
                    );
                }
            });
        });
    }

    #[cfg(target_os = "windows")]
    fn medya_tarayici_kapat(&mut self) {
        self.medya_tarayici = None;
        self.medya_tarayici_hedef = None;
        self.medya_tarayici_slug = None;
        self.medya_tarayici_url = None;
        self.medya_tarayici_hata_url = None;
        self.medya_tarayici_rect = None;
        self.medya_tarayici_rx = None;
        // Kuyruk temizlenmez: indirme tamamlandıysa tarayıcı kapansa da
        // doğrulama ve stok arşivleme devam eder.
        self.medya_tarayici_baslangic = None;
        self.medya_tarayici_son_tarama = None;
        self.medya_tarayici_gorulen.clear();
    }

    /// WebView2 indirmeyi kendi hedefine yönlendiremezse sistemin
    /// İndirilenler klasörüne düşen son videoyu sektör köküne taşır.
    #[cfg(target_os = "windows")]
    fn medya_tarayici_indirilenleri_tara(&mut self) {
        let (Some(baslangic), Some(slug)) = (
            self.medya_tarayici_baslangic,
            self.medya_tarayici_slug.clone(),
        ) else {
            return;
        };
        if self
            .medya_tarayici_son_tarama
            .is_some_and(|son| son.elapsed() < std::time::Duration::from_millis(1600))
        {
            return;
        }
        self.medya_tarayici_son_tarama = Some(Instant::now());
        let Some(aday) =
            medya_tarayici::son_indirilen_video(baslangic, &self.medya_tarayici_gorulen)
        else {
            return;
        };
        let hedef = medya_tarayici::indirme_klasoru(&slug);
        match medya_tarayici::hedefe_tasi(&aday, &hedef) {
            Ok(yol) => {
                self.medya_tarayici_gorulen.insert(aday);
                self.medya_tarayici_mesaj = format!(
                    "[i] İndirilenlerden alındı: {} · doğrulama sırasına alındı",
                    dosya_adi(&yol.to_string_lossy())
                );
                self.medya_tarayici_kuyruga_ekle(yol, &slug);
            }
            Err(hata) => self.medya_tarayici_mesaj = format!("[!] {hata}"),
        }
    }

    /// Bir indirmeyi işletme anahtarıyla kuyruğa yazar. İşletme seçili
    /// değilse dosya yine doğrulanır ve stok arşivine girer; yalnız defter
    /// eşlemesi yapılmaz.
    #[cfg(target_os = "windows")]
    fn medya_tarayici_kuyruga_ekle(&mut self, yol: std::path::PathBuf, slug: &str) {
        if !yol.is_file() {
            self.medya_tarayici_mesaj = "[!] İndirilen video dosyası bulunamadı.".to_string();
            return;
        }
        let anahtar = self
            .medya_lead
            .and_then(|idx| self.leads.get(idx))
            .map(|lead| musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum))
            .unwrap_or_default();
        self.medya_tarayici_kuyruk
            .push((yol, slug.to_string(), anahtar));
    }

    /// Tamamlanan WebView2 indirmelerini tek tek doğrulama hattına aktarır.
    #[cfg(target_os = "windows")]
    fn medya_tarayici_olaylari_isle(&mut self) {
        let Some(rx) = self.medya_tarayici_rx.take() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(medya_tarayici::Olay::Indirme { yol, basarili }) => {
                    if !basarili {
                        self.medya_tarayici_mesaj =
                            "[!] Tarayıcı indirmesi tamamlanamadı.".to_string();
                        continue;
                    }
                    let Some(yol) = yol.filter(|yol| yol.is_file()) else {
                        self.medya_tarayici_mesaj =
                            "[!] İndirme yolu alınamadı; dosyayı tarayıcıdan yeniden indirin."
                                .to_string();
                        continue;
                    };
                    // Dosya tamamlandığı anda işletmeyi sabitle: kullanıcı
                    // doğrulama sırasında başka işletmeye geçse bile bu dosya
                    // yanlış işletmenin sektörüne bağlanmaz.
                    let Some(slug) = self.medya_tarayici_slug.clone() else {
                        self.medya_tarayici_mesaj =
                            "[!] İndirme alındı ancak aktif sektör bilgisi yok.".to_string();
                        continue;
                    };
                    let hedef_klasor = medya_tarayici::indirme_klasoru(&slug);
                    let yol = if medya_tarayici::hedef_klasorde(&yol, &hedef_klasor) {
                        yol
                    } else {
                        match medya_tarayici::hedefe_tasi(&yol, &hedef_klasor) {
                            Ok(yeni) => yeni,
                            Err(hata) => {
                                self.medya_tarayici_mesaj = format!("[!] {hata}");
                                continue;
                            }
                        }
                    };
                    self.medya_tarayici_mesaj = format!(
                        "[i] İndirildi: {} · doğrulama sırasına alındı",
                        dosya_adi(&yol.to_string_lossy())
                    );
                    self.medya_tarayici_kuyruga_ekle(yol, &slug);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.medya_tarayici_rx = Some(rx);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
    }

    /// Kuyruktaki ilk indirmeyi aktif işletmenin sektörüne bağlayıp mevcut
    /// Qwen/FFmpeg hattına verir. Aynı anda tek doğrulama koşar.
    #[cfg(target_os = "windows")]
    fn medya_tarayici_kuyruk_isle(&mut self) {
        if self.medya_mesgul {
            return;
        }
        while let Some((yol, slug, anahtar)) = self.medya_tarayici_kuyruk.first().cloned() {
            self.medya_tarayici_kuyruk.remove(0);
            if !yol.is_file() {
                self.medya_tarayici_mesaj = "[!] İndirilen dosya bulunamadı.".to_string();
                continue;
            }
            self.medya_yol = yol.display().to_string();
            self.medya_tarayici_temizle = Some(yol.clone());
            self.medya_tarayici_mesaj = format!("[i] {} doğrulanıyor…", dosya_adi(&self.medya_yol));
            self.medya_yukle(slug, anahtar);
            return;
        }
    }

    /// Arka plandaki medya yükleme/doğrulama sonucunu bir kez okur.
    fn medya_isle(&mut self) {
        let Some(rx) = self.medya_islem.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(yol)) => {
                self.medya_mesgul = false;
                self.medya_yol.clear();
                self.medya_onizleme.remove(&yol);
                self.medya_mesaj = format!("[✓] Doğrulandı ve stoğa alındı: {yol}");
                if let Some(kaynak) = self.medya_tarayici_temizle.take() {
                    if kaynak.is_file() && medya_tarayici::stok_altinda(&kaynak) {
                        let _ = fs::remove_file(&kaynak);
                    }
                }
                let hedef = self.medya_hedef.take();
                let mut uygulandi = false;
                for lead in self.leads.iter_mut() {
                    let anahtar = musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum);
                    if hedef.as_deref().is_some_and(|hedef| hedef == anahtar) {
                        if !lead.medya.contains(&yol) {
                            lead.medya.push(yol.clone());
                        }
                        self.defter.medya_ayarla(&anahtar, lead.medya.clone());
                        uygulandi = true;
                        break;
                    }
                }
                if uygulandi {
                    self.defter.kaydet();
                    // Onaylanan dosya stokta kanonik `video/hero_N.mp4` ve
                    // `photo_N.jpg` adlarını oluşturur. 2 fotoğraf tamamsa tek
                    // tuş hattı bekletmeden demoyu açar; video isteğe bağlıdır.
                    if let Some(hedef) = hedef.as_deref() {
                        if let Some(idx) = self.leads.iter().position(|lead| {
                            musteriler::anahtar_uret(&lead.kaynak, &lead.name, lead.konum) == hedef
                        }) {
                            if let Some(lead) = self.leads.get(idx) {
                                let slug = sektor_medya::slug(&lead.category);
                                let secim = uretim_pipeline::hazirla(&slug);
                                self.pipeline_mesaj = secim.mesaj.clone();
                                let _ = idx;
                                // Demo hattı kaldırıldı: onaylanan dosya
                            }
                        }
                    }
                }
            }
            Ok(Err(hata)) => {
                self.medya_mesgul = false;
                if let Some(kaynak) = self.medya_tarayici_temizle.take() {
                    if kaynak.is_file() {
                        let _ = medya_tarayici::hedefe_tasi(&kaynak, Path::new(".gecici/red"));
                    }
                }
                self.medya_hedef = None;
                self.medya_oto_ac = false;
                self.medya_oto_hedef = None;
                self.medya_mesaj = format!("[!] {hata}");
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.medya_islem = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.medya_mesgul = false;
                if let Some(kaynak) = self.medya_tarayici_temizle.take() {
                    if kaynak.is_file() {
                        let _ = medya_tarayici::hedefe_tasi(&kaynak, Path::new(".gecici/red"));
                    }
                }
                self.medya_hedef = None;
                self.medya_oto_ac = false;
                self.medya_oto_hedef = None;
                self.medya_mesaj = "[!] Medya doğrulaması yarıda kaldı.".to_string();
            }
        }
    }

    /// Dosyayı arka planda Qwen2.5-VL + FFmpeg hattından geçirir ve onaylı
    /// stoğa taşır; arayüz bu sırada donmaz.
    fn medya_yukle(&mut self, slug: String, anahtar: String) {
        if self.medya_mesgul {
            return;
        }
        let yol = self.medya_yol.trim().trim_matches('"').to_string();
        if yol.is_empty() {
            self.medya_mesaj = "[!] Dosya yolu boş.".to_string();
            return;
        }
        if !Path::new(&yol).is_file() {
            self.medya_mesaj = format!("[!] Dosya bulunamadı: {yol}");
            return;
        }
        if !MedyaTuru::Foto.kabul(Path::new(&yol)) && !MedyaTuru::Video.kabul(Path::new(&yol)) {
            self.medya_mesaj =
                "[!] Geçersiz format. Fotoğraf: JPEG/PNG/WebP · Video: MP4/MOV/WebM".to_string();
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.medya_islem = Some(rx);
        self.medya_hedef = Some(anahtar);
        self.medya_mesgul = true;
        self.medya_mesaj = "[i] Qwen2.5-VL ve FFmpeg doğrulaması sürüyor…".to_string();
        std::thread::spawn(move || {
            let sonuc = (|| -> Result<String, String> {
                let veri = fs::read(&yol).map_err(|hata| format!("{yol} okunamadı: {hata}"))?;
                let ad = Path::new(&yol)
                    .file_name()
                    .map(|ad| ad.to_string_lossy().to_string())
                    .ok_or_else(|| "dosya adı yok".to_string())?;
                let ekleme = medya_manuel::dosya_ekle(Path::new("."), &slug, &ad, "", &veri)?;
                let bekleyen = format!("{}/{}/{}", medya_onay::BEKLEYEN, slug, ekleme.dosya);
                medya_onay::onayla_dosya(Path::new("."), &slug, &bekleyen)?;
                Ok(format!(
                    "{}/{}/{}",
                    sektor_medya::KLASOR,
                    slug,
                    ekleme.dosya
                ))
            })();
            let _ = tx.send(sonuc);
        });
    }

    /// Köprü olaylarını arayüze aktarır (her karede çağrılır).
    pub(crate) fn gelen_olaylari_isle(&mut self) {
        while let Ok(olay) = self.gelen_rx.try_recv() {
            match olay {
                gelen::Olay::Gelen(kayit) => {
                    self.logs.push(format!(
                        "[✓] Gelen mesaj · {} · {}",
                        kayit.baslik(),
                        kayit.niyet.etiket()
                    ));
                    self.gelen.ekle(kayit);
                    self.gelen.kaydet();
                }
                gelen::Olay::Giden(kayit) => {
                    self.logs
                        .push(format!("[i] Köprü mesajı iletti · {}", kayit.baslik()));
                    self.gelen.ekle(kayit);
                    self.gelen.kaydet();
                }
                gelen::Olay::Uyari(uyari) => {
                    self.kopru.mesaj = uyari.clone();
                    self.logs.push(format!("[i] {uyari}"));
                }
            }
        }
    }

    /// Alan adı sorgusunun sonucunu toplar.
    pub(crate) fn domain_sonucu_al(&mut self) {
        let Some(rx) = &self.domain_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(liste) => {
                let musait = liste
                    .iter()
                    .filter(|sonuc| sonuc.durum == domain::Durum::Musait)
                    .count();
                self.logs.push(format!(
                    "[✓] Alan adı sorgusu bitti: {musait} uzantı müsait."
                ));
                self.domain_sonuc = liste;
                self.domain_rx = None;
                self.domain_calisiyor = false;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.domain_rx = None;
                self.domain_calisiyor = false;
            }
        }
    }

    /// Yerel köprüyü bir kez başlatır (Katman 5'e ilk girişte).
    pub(crate) fn kopruyu_baslat(&mut self) {
        if self.gelen_durdur.is_some() {
            return;
        }
        let (port, acildi, durdur) =
            gelen::dinleyici_baslat_esnek(self.gelen.port(), self.gelen_tx.clone());
        self.gelen.port_ayarla(port);
        self.kopru.port = port;
        self.kopru.calisiyor = acildi;
        self.kopru.mesaj = if acildi {
            format!("127.0.0.1:{port} dinleniyor")
        } else {
            "dinleyici açılamadı; elle yapıştırma kullanılabilir".to_string()
        };
        self.kopru_son = Some(Instant::now());
        self.gelen_durdur = Some(durdur);
    }

    /// Dinleyici düştüyse boş portta yeniden kurar (auto-reconnect).
    ///
    /// Köprü süreci kapanır, port başka programa geçer ya da ağ katmanı
    /// sıfırlanırsa uygulama kendini toparlar; kullanıcı uygulamayı yeniden
    /// başlatmak zorunda kalmaz.
    fn kopru_yeniden_baglan(&mut self) {
        if let Some(durdur) = self.gelen_durdur.take() {
            durdur.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let (port, acildi, durdur) =
            gelen::dinleyici_baslat_esnek(self.gelen.port(), self.gelen_tx.clone());
        self.gelen.port_ayarla(port);
        self.kopru.port = port;
        self.kopru.calisiyor = acildi;
        self.kopru.mesaj = if acildi {
            format!("127.0.0.1:{port} yeniden bağlandı")
        } else {
            "köprü yeniden açılamadı; elle yapıştırma kullanılabilir".to_string()
        };
        self.gelen_durdur = Some(durdur);
    }

    /// Köprünün ayakta olup olmadığını 3 saniyede bir yoklar.
    pub(crate) fn kopru_tazele(&mut self) {
        let gerekli = match self.kopru_son {
            Some(zaman) => zaman.elapsed().as_secs() >= 3,
            None => true,
        };
        if gerekli {
            self.kopru.calisiyor = gelen::kopru_ayakta(self.gelen.port());
            if !self.kopru.calisiyor {
                self.kopru_yeniden_baglan();
            }
            self.kopru_son = Some(Instant::now());
        }
    }
}

// ---------------------------------------------------------------------------
// Katman 3 — canlı demo önizleme
// ---------------------------------------------------------------------------

impl LeadApp {}

/// Test edilebilir kök parametreli tarama.
fn medya_stok_listesi_yoldan(kok: &Path, slug: &str) -> Vec<String> {
    let kok = kok.join(slug);
    let mut dosyalar = Vec::new();
    medya_klasoru_tara(&kok, 0, &mut dosyalar);
    dosyalar.sort();
    dosyalar.dedup();
    dosyalar
}

/// `media/stock/<slug>/` altındaki tüm alt klasörleri tarar. Yalnız birkaç
/// seviye inilir; sembolik bağ döngüsü olsa bile tarama sonsuza gitmez.
fn medya_klasoru_tara(dizin: &Path, derinlik: usize, cikti: &mut Vec<String>) {
    if derinlik > 4 {
        return;
    }
    let Ok(girisler) = fs::read_dir(dizin) else {
        return;
    };
    for giris in girisler.flatten() {
        let ad = giris.file_name();
        if ad.to_string_lossy().starts_with('.') {
            continue;
        }
        let yol = giris.path();
        if yol.is_dir() {
            medya_klasoru_tara(&yol, derinlik + 1, cikti);
            continue;
        }
        if !yol.is_file() {
            continue;
        }
        let uzanti = yol
            .extension()
            .and_then(|uzanti| uzanti.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(
            uzanti.as_str(),
            "mp4"
                | "mov"
                | "webm"
                | "m4v"
                | "mkv"
                | "avi"
                | "jpg"
                | "jpeg"
                | "png"
                | "webp"
                | "gif"
                | "bmp"
        ) {
            cikti.push(yol.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Arşiv imzası: yol + boyut + değişme zamanı. Yeni indirme veya aynı adlı
/// dosyanın yenilenmesi thumbnail önbelleğini tazeler.
fn medya_stok_imzasi(dosyalar: &[String]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for dosya in dosyalar {
        dosya.hash(&mut hasher);
        if let Ok(meta) = fs::metadata(dosya) {
            meta.len().hash(&mut hasher);
            if let Ok(degisti) = meta.modified() {
                if let Ok(zaman) = degisti.duration_since(std::time::UNIX_EPOCH) {
                    zaman.as_nanos().hash(&mut hasher);
                }
            }
        }
    }
    hasher.finish()
}

/// Seçili medyayı kısa metinle özetler: "2 fotoğraf" / "1 video + 2 fotoğraf".
fn medya_ozeti(medya: &[String]) -> String {
    let video = medya.iter().filter(|yol| medya_video(yol)).count();
    let foto = medya.len().saturating_sub(video);
    match (video, foto) {
        (0, foto) => format!("{foto} fotoğraf"),
        (video, 0) => format!("{video} video"),
        (video, foto) => format!("{video} video + {foto} fotoğraf"),
    }
}

/// Video için poster bulur; yoksa ffmpeg ile 480×270 JPEG thumbnail üretir.
/// Üretilen dosya `.gecici/medya_onizleme/` altında yol+yaş anahtarıyla
/// saklanır; her karede yeniden üretilmez.
fn medya_video_onizleme(video: &Path) -> Option<std::path::PathBuf> {
    if let Some(poster) = medya_poster(video) {
        return Some(poster);
    }
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    video.to_string_lossy().hash(&mut hasher);
    if let Ok(meta) = fs::metadata(video) {
        meta.len().hash(&mut hasher);
        if let Ok(degisti) = meta.modified() {
            if let Ok(zaman) = degisti.duration_since(std::time::UNIX_EPOCH) {
                zaman.as_nanos().hash(&mut hasher);
            }
        }
    }
    let klasor = Path::new(".gecici/medya_onizleme");
    let hedef = klasor.join(format!("{:016x}.jpg", hasher.finish()));
    if hedef.is_file() {
        return Some(hedef);
    }
    fs::create_dir_all(klasor).ok()?;
    let durum = crate::stok_video::gizli_komut("ffmpeg")
        .args(["-y", "-v", "error", "-ss", "0.4", "-i"])
        .arg(video)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale=480:270:force_original_aspect_ratio=decrease,pad=480:270:(ow-iw)/2:(oh-ih)/2",
            "-q:v",
            "3",
        ])
        .arg(&hedef)
        .status()
        .ok()?;
    (durum.success() && hedef.is_file()).then_some(hedef)
}

/// Windows'un yerel dosya seçme penceresi. Yalnızca istenen türü gösterir.
#[cfg(target_os = "windows")]
fn medya_dosya_sec(tur: MedyaTuru) -> Option<std::path::PathBuf> {
    let mut secim = rfd::FileDialog::new()
        .set_title(format!("{} seç", tur.baslik()))
        .add_filter(tur.baslik(), tur.uzantilar());
    if let Ok(dizin) = std::env::current_dir() {
        secim = secim.set_directory(dizin);
    }
    secim.pick_file()
}

#[cfg(not(target_os = "windows"))]
fn medya_dosya_sec(_tur: MedyaTuru) -> Option<std::path::PathBuf> {
    None
}

/// "Gözat..." için tüm desteklenen medya türlerini gösteren yerel pencere.
#[cfg(target_os = "windows")]
fn medya_dosya_sec_genel() -> Option<std::path::PathBuf> {
    let mut secim = rfd::FileDialog::new()
        .set_title("Medya dosyası seç")
        .add_filter(
            "Medya",
            &["jpg", "jpeg", "png", "webp", "mp4", "mov", "webm"],
        )
        .add_filter("Fotoğraf", &["jpg", "jpeg", "png", "webp"])
        .add_filter("Video", &["mp4", "mov", "webm"]);
    if let Ok(dizin) = std::env::current_dir() {
        secim = secim.set_directory(dizin);
    }
    secim.pick_file()
}

#[cfg(not(target_os = "windows"))]
fn medya_dosya_sec_genel() -> Option<std::path::PathBuf> {
    None
}

/// Arşiv dosyasını `.gecici/red/` altına taşır. Yalnız `media/stock/`
/// altındaki dosyalar kaldırılabilir; demo/çıktı klasörlerine dokunulmaz.
fn medya_arsivden_kaldir(
    kok: &Path,
    goreli: &str,
    red: &Path,
) -> Result<std::path::PathBuf, String> {
    let onek = format!("{}/", sektor_medya::KLASOR);
    if !goreli.starts_with(&onek) {
        return Err(format!("{goreli}: yalnız stok arşivi kaldırılabilir"));
    }
    let kaynak = kok.join(goreli);
    if !kaynak.is_file() {
        return Err(format!("{goreli}: dosya bulunamadı"));
    }
    medya_tarayici::hedefe_tasi(&kaynak, red)
}

/// Medya kartı etkileşim sonucu: seçim değişti, büyük önizleme istendi,
/// dosya kaldırılmak istendi.
struct KartYaniti {
    secildi: bool,
    buyut: bool,
    sil: bool,
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn stok_arsivi_alt_klasorleri_ve_medya_turlerini_tarar() {
        // Kendi klasör adı: `medya_arsiv` testleriyle aynı dizini paylaşırsa
        // paralel çalışan testler birbirinin dosyalarını siler.
        let kok = std::env::temp_dir().join(format!("nexus_katman_arsiv_{}", std::process::id()));
        let _ = fs::remove_dir_all(&kok);
        fs::create_dir_all(kok.join("cekici/video")).unwrap();
        fs::create_dir_all(kok.join("cekici/photos/alt")).unwrap();
        fs::create_dir_all(kok.join("cekici/fotolar")).unwrap();
        fs::write(kok.join("cekici/video/hero_1.mp4"), [1u8; 8]).unwrap();
        fs::write(kok.join("cekici/photos/alt/photo_2.jpg"), [2u8; 8]).unwrap();
        fs::write(kok.join("cekici/fotolar/3.webp"), [3u8; 8]).unwrap();
        fs::write(kok.join("cekici/not.txt"), [4u8; 8]).unwrap();

        let dosyalar = medya_stok_listesi_yoldan(&kok, "cekici");
        assert_eq!(
            dosyalar.len(),
            3,
            "alt klasörler ve geçerli medya taranmalı"
        );
        assert!(dosyalar.iter().any(|yol| yol.ends_with("hero_1.mp4")));
        assert!(dosyalar.iter().any(|yol| yol.ends_with("photo_2.jpg")));
        assert!(dosyalar.iter().any(|yol| yol.ends_with("3.webp")));
        assert!(!dosyalar.iter().any(|yol| yol.ends_with("not.txt")));
        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn arsiv_dosyasi_red_klasorune_tasinir() {
        let kok = std::env::temp_dir().join(format!("nexus_sil_{}", std::process::id()));
        let _ = fs::remove_dir_all(&kok);
        fs::create_dir_all(kok.join("media/stock/cekici/photos")).unwrap();
        let red = kok.join(".gecici/red");
        let dosya = kok.join("media/stock/cekici/photos/1.jpg");
        fs::write(&dosya, [1u8; 8]).unwrap();

        let hedef = medya_arsivden_kaldir(&kok, "media/stock/cekici/photos/1.jpg", &red).unwrap();
        assert!(!dosya.exists(), "kaynak arşivden kalkmalı");
        assert!(
            hedef.starts_with(&red) && hedef.is_file(),
            "red klasörüne taşınmalı"
        );
        assert!(
            medya_arsivden_kaldir(&kok, "demos/cekici.html", &red).is_err(),
            "stok dışı yol reddedilmeli"
        );
        assert!(
            medya_arsivden_kaldir(&kok, "media/stock/cekici/photos/yok.jpg", &red).is_err(),
            "olmayan dosya hata döner"
        );
        let _ = fs::remove_dir_all(&kok);
    }

    #[test]
    fn arsiv_imzasi_yeni_dosyada_degisir() {
        let kok = std::env::temp_dir().join(format!("nexus_imza_{}", std::process::id()));
        let _ = fs::remove_dir_all(&kok);
        fs::create_dir_all(kok.join("cekici/video")).unwrap();
        let ilk = kok.join("cekici/video/hero_1.mp4");
        fs::write(&ilk, [1u8; 8]).unwrap();
        let once = medya_stok_imzasi(&[ilk.to_string_lossy().to_string()]);
        let ikinci = kok.join("cekici/video/hero_2.mp4");
        fs::write(&ikinci, [2u8; 8]).unwrap();
        let sonra = medya_stok_imzasi(&[
            ilk.to_string_lossy().to_string(),
            ikinci.to_string_lossy().to_string(),
        ]);
        assert_ne!(once, sonra);
        let _ = fs::remove_dir_all(&kok);
    }
}

impl LeadApp {
    /// Dosya yolundan küçük bir önizleme dokusu üretir ve önbelleğe alır.
    /// Video için aynı adlı poster, yoksa `ffmpeg` ile üretilen thumbnail.
    fn medya_doku(&mut self, ctx: &egui::Context, yol: &str) -> Option<egui::TextureHandle> {
        self.medya_doku_boyut(ctx, yol, 480, 270)
    }

    /// Belirtilen en büyük ölçüde önizleme dokusu. Küçük kartlar 480×270,
    /// büyük önizleme 1280×720 ister; ikisi ayrı önbellek anahtarı kullanır.
    fn medya_doku_boyut(
        &mut self,
        ctx: &egui::Context,
        yol: &str,
        max_w: u32,
        max_h: u32,
    ) -> Option<egui::TextureHandle> {
        let anahtar = format!("{yol}|{max_w}x{max_h}");
        if let Some(doku) = self.medya_onizleme.get(&anahtar) {
            return doku.clone();
        }
        let kaynak = if medya_video(yol) {
            medya_video_onizleme(Path::new(yol))
        } else {
            Some(Path::new(yol).to_path_buf())
        };
        let doku = kaynak.and_then(|kaynak| {
            let resim = image::open(&kaynak).ok()?.thumbnail(max_w, max_h);
            let rgba = resim.to_rgba8();
            let boyut = [rgba.width() as usize, rgba.height() as usize];
            let renk = egui::ColorImage::from_rgba_unmultiplied(boyut, rgba.as_raw());
            Some(ctx.load_texture(
                format!("medya:{anahtar}:{}", kaynak.display()),
                renk,
                egui::TextureOptions::LINEAR,
            ))
        });
        self.medya_onizleme.insert(anahtar, doku.clone());
        doku
    }
}

/// `#rrggbb` → renk; bozuk/boş değer varsayılana döner.
pub(crate) fn renk_coz(metin: &str, varsayilan: egui::Color32) -> egui::Color32 {
    let temiz = metin.trim().trim_start_matches('#');
    if temiz.len() != 6 {
        return varsayilan;
    }
    let kanal = |baslangic: usize| u8::from_str_radix(&temiz[baslangic..baslangic + 2], 16).ok();
    match (kanal(0), kanal(2), kanal(4)) {
        (Some(r), Some(g), Some(b)) => egui::Color32::from_rgb(r, g, b),
        _ => varsayilan,
    }
}

/// Kota şeridindeki vurgulu düğme (API & Kota Yönetim).
/// Amber tonu: profil denetiminde "iyileştir" durumu.
const KEHRIBAR: egui::Color32 = crate::katman34::KEHRIBAR;

/// Kota sıfırlanmasına kalan süre: "2sa 14dk" ya da "14dk 03sn".
fn geri_sayim(saniye: i64) -> String {
    let saniye = saniye.max(0);
    let saat = saniye / 3600;
    let dakika = (saniye % 3600) / 60;
    if saat > 0 {
        format!("{saat}sa {dakika:02}dk")
    } else {
        format!("{dakika}dk {:02}sn", saniye % 60)
    }
}

fn api_dugmesi(metin: &str) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(metin.to_string())
            .size(9.0)
            .color(egui::Color32::from_rgb(11, 19, 31)),
    )
    .fill(VURGU)
    .rounding(egui::Rounding::same(6.0))
    .min_size(egui::vec2(0.0, 24.0))
}
