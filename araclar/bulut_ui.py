"""Kota şeridi ve API panelini bulut senkronu + güvenlik marjı ile genişletir."""

import sys
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def oku(yol):
    return Path(yol).read_text(encoding="utf-8")


def yaz(yol, metin):
    Path(yol).write_text(metin, encoding="utf-8", newline="")


def degistir(metin, eski, yeni):
    assert metin.count(eski) == 1, f"beklenen 1, bulunan {metin.count(eski)}: {eski[:80]!r}"
    return metin.replace(eski, yeni)


k = oku("src/katmanlar.rs")

# --- 1) Kota şeridi: güvenlik marjı, bulut sayacı, geri sayım, onay düğmesi ---
k = degistir(
    k,
    """                    pill(ui, "GOOGLE MAPS API", VURGU);
                    let (durum, renk) = match ozet.seviye {
                        google::Seviye::Normal => ("kota normal", YESIL),
                        google::Seviye::Sari => ("ALARM %80", TURUNCU),
                        google::Seviye::Kirmizi => ("LİMİT AŞILDI", KIRMIZI),
                    };
                    pill(ui, durum, renk);
                    ui.label(
                        egui::RichText::new(format!(
                            "oturum {} istek · günlük {}/{} · aylık {}/{} (%{:.1}) · kalan {} · ücretsiz kota %{:.1}",
                            ozet.oturum,
                            ozet.gunluk,
                            ozet.gunluk_limit,
                            ozet.aylik,
                            ozet.aylik_limit,
                            ozet.ay_yuzde * 100.0,
                            ozet.kalan_ay,
                            ozet.ucretsiz_yuzde * 100.0
                        ))
                        .size(9.5)
                        .color(if ozet.seviye == google::Seviye::Kirmizi {
                            KIRMIZI
                        } else {
                            DIM
                        }),
                    );
                    if kasa.anahtar().is_none() {
                        pill(ui, "anahtar yok", TURUNCU);
                    }""",
    """                    pill(ui, "GOOGLE MAPS API", VURGU);
                    let (durum, renk) = match ozet.seviye {
                        google::Seviye::Normal => ("kota normal", YESIL),
                        google::Seviye::Sari => ("ALARM %80", TURUNCU),
                        google::Seviye::Kirmizi => ("LİMİT AŞILDI", KIRMIZI),
                    };
                    pill(ui, durum, renk);
                    if ozet.manuel_gerekli {
                        pill(
                            ui,
                            &format!("GÜVENLİK MARJI {}/{}", ozet.gunluk, ozet.gunluk_limit),
                            TURUNCU,
                        );
                    }
                    if let Some(bulut) = ozet.bulut_gunluk {
                        pill(ui, &format!("bulut {bulut} istek"), VURGU);
                    }
                    ui.label(
                        egui::RichText::new(format!(
                            "oturum {} · günlük {}/{} · aylık {}/{} · kalan {} · ücretsiz %{:.2} · sıfırlanma {}",
                            ozet.oturum,
                            ozet.gunluk,
                            ozet.gunluk_limit,
                            ozet.aylik,
                            ozet.aylik_limit,
                            ozet.kalan_ay,
                            ozet.ucretsiz_yuzde * 100.0,
                            geri_sayim(ozet.sifirlanma)
                        ))
                        .size(9.5)
                        .color(if ozet.seviye == google::Seviye::Kirmizi {
                            KIRMIZI
                        } else {
                            DIM
                        }),
                    );
                    if kasa.anahtar().is_none() {
                        pill(ui, "anahtar yok", TURUNCU);
                    } else if ozet.manuel_gerekli
                        && ui
                            .add(api_dugmesi(&format!(
                                "KALAN {} İSTEĞİ KULLAN",
                                ozet.gunluk_limit.saturating_sub(ozet.gunluk)
                            )))
                            .clicked()
                    {
                        self.kota_onay = true;
                        self.api_mesaj = format!(
                            "Onay verildi: kalan {} istek elle kullanılacak.",
                            ozet.gunluk_limit.saturating_sub(ozet.gunluk)
                        );
                    }""",
)

k = degistir(
    k,
    """        if self.is_running || self.api_panel {
            ctx.request_repaint_after(Duration::from_millis(500));
        }""",
    """        if self.is_running || self.api_panel || ozet.manuel_gerekli {
            ctx.request_repaint_after(Duration::from_millis(500));
        }""",
)

# --- 2) Modal: proje kimliği + bulut senkronu ---------------------------------
k = degistir(
    k,
    """                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.add(
                    egui::ProgressBar::new(ozet.gun_yuzde as f32)""",
    """                ui.add_space(10.0);
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
                        "Bulut: bugün {gun} · bu ay {ay} istek · son senkron {zaman} · 60 sn'de bir"
                    ),
                    _ => "Bulut verisi bekleniyor: proje kimliği + API anahtarı gerekir (60 sn'de bir senkron)."
                        .to_string(),
                };
                ui.label(egui::RichText::new(bulut_metni).size(9.0).color(DIM));
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
                    egui::ProgressBar::new(ozet.gun_yuzde as f32)""",
)

# --- 3) Canlı istek satırı ----------------------------------------------------
k = degistir(
    k,
    """                ui.label(
                    egui::RichText::new(format!(
                        "Oturumda atılan istek: {} · kurulumdan bu yana: {}",
                        ozet.oturum, ozet.toplam
                    ))
                    .size(9.5)
                    .color(DIM),
                );""",
    """                ui.label(
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
                );""",
)

# --- 4) Güvenlik marjı uyarı kutusu ------------------------------------------
k = degistir(
    k,
    """                if ozet.seviye != google::Seviye::Normal {
                    ui.label(
                        egui::RichText::new(
                            "⚠ Maliyet uyarısı: istek sınırına yaklaşıldı; yeni taramalar duracak.",
                        )
                        .size(9.5)
                        .color(KIRMIZI),
                    );
                }""",
    """                if ozet.manuel_gerekli {
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
                }""",
)

# --- 5) Manuel senkron sonucu -------------------------------------------------
k = degistir(
    k,
    """    /// Katman 1'in ortasındaki interaktif harita (OSM döşemeleri).""",
    """    /// Arka plandaki "Şimdi senkronla" isteğinin sonucunu bir kez okur.
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

    /// Katman 1'in ortasındaki interaktif harita (Google Places noktaları).""",
)

# --- 6) Geri sayım yardımcısı -------------------------------------------------
k = degistir(
    k,
    """/// Kota şeridindeki vurgulu düğme (API & Kota Yönetim).""",
    """/// Saniyeyi SS:DD:SS geri sayımına çevirir (UTC gece yarısına kalan).
fn geri_sayim(saniye: i64) -> String {
    let s = saniye.max(0);
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

/// Kota şeridindeki vurgulu düğme (API & Kota Yönetim).""",
)

yaz("src/katmanlar.rs", k)
print("katmanlar.rs güncellendi")
