"""Keşif & Radar ekranı: web sitesi olan işletmeleri ele, yerleşimi düzenle.

Tek seferlik geçiş betiği; main.rs üzerinde birebir metin değişimi yapar ve
her adımda beklenen eşleşme sayısını doğrular.
"""

from pathlib import Path

YOL = Path("src/main.rs")
s = YOL.read_text(encoding="utf-8")
n0 = len(s)
adim = []


def rep(eski: str, yeni: str, adet: int = 1, ad: str = "") -> None:
    global s
    c = s.count(eski)
    assert c == adet, f"{ad}: beklenen {adet}, bulunan {c}"
    s = s.replace(eski, yeni)
    adim.append(ad)


# --- 1) Web sitesi filtresi: mutlak eleme ----------------------------------
rep(
    """/// İşletmenin web varlığı. Site yaptırmaya niyet göstergesidir: sosyal medya
/// hesabı olup sitesi olmayan işletme, çevrimiçi görünmek istediğini zaten
/// göstermiş ama henüz site yaptırmamış demektir — en sıcak hedef kitle.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
enum SiteDurumu {
    /// Hiçbir web etiketi yok.
    Yok,
    /// Yalnızca Instagram/Facebook/Linktree gibi bir sosyal medya adresi var.
    Sosyal,
    /// Google business.site, WixSite, Blogspot gibi ücretsiz bir araçla kurulmuş site.
    Ucretsiz,
}

""",
    "",
    ad="SiteDurumu enum kaldırıldı",
)

rep(
    """    /// Web varlığı: site yok / yalnızca sosyal medya / ücretsiz site aracı.
    site: SiteDurumu,
    /// Bulunan sosyal medya veya ücretsiz site adresi.
    site_linki: String,
""",
    """    /// Kayıtta görülen web adresi. Bu ekran yalnızca web sitesi olmayan
    /// işletmeleri listelediği için yeni kayıtlarda boş kalır; eski önbellek
    /// kayıtlarındaki değer yalnızca kaydın elenmesi için okunur.
    site_linki: String,
""",
    ad="BusinessLead.site alanı kaldırıldı",
)

rep(
    """                            match lead.site {
                                SiteDurumu::Sosyal => pill(ui, "sosyal medya var", MAVI),
                                SiteDurumu::Ucretsiz => pill(ui, "ücretsiz site", MAVI),
                                SiteDurumu::Yok => {}
                            }
""",
    "",
    ad="kart rozetleri",
)

rep(
    """                                    match lead.site {
                                        SiteDurumu::Sosyal => pill(ui, "sosyal medya var", MAVI),
                                        SiteDurumu::Ucretsiz => pill(ui, "ücretsiz site", MAVI),
                                        SiteDurumu::Yok => {}
                                    }
""",
    "",
    ad="detay rozetleri",
)

rep(
    """                            let baslik = match lead.site {
                                SiteDurumu::Sosyal => "SOSYAL MEDYA",
                                SiteDurumu::Ucretsiz => "ÜCRETSİZ SİTE",
                                SiteDurumu::Yok => "WEB ADRESİ",
                            };
                            ui.label(
                                egui::RichText::new(baslik)
""",
    """                            ui.label(
                                egui::RichText::new("WEB ADRESİ")
""",
    ad="detay web adresi başlığı",
)

rep(
    """                            site_kodu(lead.site),
""",
    """                            "yok",
""",
    ad="defter site kodu",
)

rep(
    """            // Google'da da aynı ayrım: gerçek alan adı olan elenir, sosyal
            // medya / ücretsiz site adresi olan sıcak fırsat olarak tutulur.
            let site_uri = yer
                .get("websiteUri")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let site = if site_uri.is_empty() {
                SiteDurumu::Yok
            } else {
                match site_durumu_bul(site_uri) {
                    Some(durum) => durum,
                    None => {
                        siteli += 1;
                        continue;
                    }
                }
            };
""",
    """            // Bu ekranın tek hedefi dijital varlığı olmayan işletmeler: Google
            // kaydında web adresi olan (kendi alan adı, sosyal medya veya
            // ücretsiz site aracı) her işletme listeden çıkarılır.
            let site_uri = yer
                .get("websiteUri")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if !site_uri.is_empty() {
                siteli += 1;
                continue;
            }
""",
    ad="google_isle siteli eleme",
)

rep(
    """                kayit_dolulugu: 0,
                site,
                site_linki: site_uri.to_string(),
""",
    """                kayit_dolulugu: 0,
                site_linki: String::new(),
""",
    ad="BusinessLead kurulumu",
)

rep(
    """        Some(SonucOnbellekKaydi {
            leads: kayit.leads.clone(),
            gun,
        })
""",
    """        // Eski önbellek kayıtları web sitesi olan işletmeler içerebilir; bu
        // ekranın kuralı gereği yalnızca sitesiz kayıtlar geri döner.
        let leads: Vec<BusinessLead> = kayit
            .leads
            .iter()
            .filter(|lead| lead.site_linki.is_empty())
            .cloned()
            .collect();
        Some(SonucOnbellekKaydi { leads, gun })
""",
    ad="önbellek sitesiz süzgeci",
)

# site yardımcıları (url_alan_adi, site_kodu, site_durumu_bul) tamamen kalkar
satirlar = s.split("\n")
i = next(k for k, l in enumerate(satirlar) if "fn url_alan_adi" in l)
bas = i
while bas > 0 and (satirlar[bas - 1].startswith("/// ") or satirlar[bas - 1].startswith("#[")):
    bas -= 1
j = next(k for k, l in enumerate(satirlar) if "fn ilce_kotasi" in l)
son = j
while satirlar[son - 1].startswith("/// ") or satirlar[son - 1].startswith("#["):
    son -= 1
del satirlar[bas:son]
s = "\n".join(satirlar)
adim.append("site yardımcıları silindi")

# --- 2) Arayüz: eski/tekrarlı öğeler --------------------------------------
rep(
    """                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("canlı OSM verisi")
                                .size(10.5)
                                .color(DIM),
                        );
                        ui.add_space(6.0);
                        let (nokta, _) =
                            ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
                        ui.painter().circle_filled(nokta.center(), 3.0, YESIL);
""",
    "",
    ad="üst bardaki OSM etiketi",
)

rep(
    """                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if google_pili(ui, &self.kota_durum).clicked() {
                            self.anahtar_formunu_ac();
                        }
                        ui.add_space(6.0);
                        kontur_pill(ui, "Google Places · canlı");
                        ui.add_space(6.0);
                        kontur_pill(ui, &format!("{} sonuç", self.leads.len()));
                    });
""",
    """                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        kontur_pill(ui, "Google Places · canlı");
                    });
""",
    ad="orta başlıktaki tekrar rozetler",
)

# --- 3) Yerleşim: paneller, harita, araç çubukları ------------------------
rep(
    """            .exact_width(330.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(18.0, 16.0)),
""",
    """            .exact_width(300.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(16.0, 14.0)),
""",
    ad="sol filtre genişliği ve boşluğu",
)
rep(
    """                    .inner_margin(egui::Margin::symmetric(22.0, 16.0)),
""",
    """                    .inner_margin(egui::Margin::symmetric(16.0, 14.0)),
""",
    ad="orta panel iç boşluğu",
)
rep(
    """        egui::SidePanel::right("kesif_harita")
            .resizable(true)
            .default_width(400.0)
            .min_width(320.0)
            .max_width(680.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14.0, 14.0)),
            )
""",
    """        // Harita ekranın en büyük bileşeni: pencere genişliğinin ~%42'si,
        // 420 px'in altına inmez; kullanıcı sürükleyerek 760 px'e çıkarabilir.
        let harita_genisligi = (ctx.screen_rect().width() * 0.42).clamp(420.0, 760.0);
        egui::SidePanel::right("kesif_harita")
            .resizable(true)
            .default_width(harita_genisligi)
            .min_width(360.0)
            .max_width(760.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(12.0, 12.0)),
            )
""",
    ad="harita paneli genişliği",
)
rep(
    """                ui.add_space(8.0);
                let yukseklik = (ui.available_height() - 10.0).max(240.0);
""",
    """                ui.add_space(6.0);
                let yukseklik = ui.available_height().max(240.0);
""",
    ad="harita yüksekliği",
)

# araç çubuğu: sağa sabit hizalama yerine sarmalı akış (dar pencerede taşmaz)
rep(
    """                    .count();
                ui.horizontal(|ui| {
                    let arama_alani = 250.0_f32.min(ui.available_width() * 0.4);
""",
    """                    .count();
                ui.horizontal_wrapped(|ui| {
                    let arama_alani = 240.0_f32.min(ui.available_width() * 0.5);
""",
    ad="araç çubuğu sarmalı",
)
rep(
    """                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let demo_metin = if secili > 0 {
""",
    """                    ui.add_space(6.0);
                    {
                        let demo_metin = if secili > 0 {
""",
    ad="demo düğmesi hizası",
)
rep(
    """                        if hayalet_buton(ui, "Demos klasörünü aç").clicked() {
                            let _ = fs::create_dir_all(DEMO_KLASORU);
                            if let Err(hata) = klasoru_ac(DEMO_KLASORU) {
                                self.logs
                                    .push(format!("[!] Demos klasörü açılamadı: {hata}"));
                            }
                        }
                    });
                });
""",
    """                        if hayalet_buton(ui, "Demos klasörünü aç").clicked() {
                            let _ = fs::create_dir_all(DEMO_KLASORU);
                            if let Err(hata) = klasoru_ac(DEMO_KLASORU) {
                                self.logs
                                    .push(format!("[!] Demos klasörü açılamadı: {hata}"));
                            }
                        }
                    }
                });
""",
    ad="araç çubuğu kapanışı",
)

# liste şeridi: sarmalı akış + tekrar eden WhatsApp sayacı kaldırıldı
rep(
    """                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new("LİSTE").size(9.5).color(DIM));
""",
    """                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new("LİSTE").size(9.5).color(DIM));
""",
    ad="liste şeridi sarmalı",
)
rep(
    """                    // Sağda: bugünkü gönderim hakkı ve takip kuyruğu.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let kuyruk = self.defter.takip_edilecekler(&bugun, TAKIP_GUN).len();
                        if hayalet_buton(ui, &format!("Takip kuyruğu ({kuyruk})")).clicked() {
                            self.takip_panel_acik = true;
                        }
                        ui.add_space(10.0);
                        let kalan_hedef = self.hedef.kalan();
                        if hayalet_buton(
                            ui,
                            &format!("Günlük hedef ({kalan_hedef}/{})", hedef::GUNLUK_HEDEF),
                        )
                        .clicked()
                        {
                            self.bugun_panel_acik = true;
                        }
                        ui.add_space(10.0);
                        let gunluk = self.defter.gun_gonderim(&bugun);
                        ui.label(
                            egui::RichText::new(format!(
                                "bugün {gunluk}/{GUNLUK_GONDERIM_LIMITI} mesaj"
                            ))
                            .size(10.5)
                            .color(
                                if gunluk >= GUNLUK_GONDERIM_LIMITI {
                                    VURGU
                                } else {
                                    DIM
                                },
                            ),
                        );
                    });
""",
    """                    ui.add_space(12.0);
                    let kuyruk = self.defter.takip_edilecekler(&bugun, TAKIP_GUN).len();
                    if hayalet_buton(ui, &format!("Takip kuyruğu ({kuyruk})")).clicked() {
                        self.takip_panel_acik = true;
                    }
                    let kalan_hedef = self.hedef.kalan();
                    if hayalet_buton(ui, &format!("Hedef ({kalan_hedef}/{})", hedef::GUNLUK_HEDEF))
                        .clicked()
                    {
                        self.bugun_panel_acik = true;
                    }
""",
    ad="liste şeridi sayaçları",
)

# --- 4) Test: sosyal medya adresi de elenir ------------------------------
rep(
    """            {"id":"def","displayName":{"text":"Siteli Kuaför"},"websiteUri":"https://ornek.com"},
            {"id":"ghi","displayName":{"text":"Gece Kuaförü"},"formattedAddress":"X","nationalPhoneNumber":"0555 000 00 00"}""",
    """            {"id":"def","displayName":{"text":"Siteli Kuaför"},"websiteUri":"https://ornek.com"},
            {"id":"jkl","displayName":{"text":"Instagram Kuaför"},"websiteUri":"https://instagram.com/kuafoor","nationalPhoneNumber":"0555 111 11 11"},
            {"id":"ghi","displayName":{"text":"Gece Kuaförü"},"formattedAddress":"X","nationalPhoneNumber":"0555 000 00 00"}""",
    ad="test verisi",
)
rep(
    """        assert_eq!(siteli, 1, "web siteli kayıt ayıklandı");
        assert_eq!(liste.len(), 2);
""",
    """        // Kendi alan adı da sosyal medya adresi de listeye girmez.
        assert_eq!(siteli, 2, "web adresi olan kayıtlar ayıklandı");
        assert_eq!(liste.len(), 2);
        assert!(liste.iter().all(|lead| lead.site_linki.is_empty()));
        assert!(!liste.iter().any(|lead| lead.name == "Instagram Kuaför"));
""",
    ad="test beklentileri",
)

for kalinti in ["SiteDurumu", "site_durumu_bul", "url_alan_adi", "site_kodu", "canlı OSM"]:
    for no, satir in enumerate(s.split("\n"), 1):
        if kalinti in satir:
            print(f"KALINTI {kalinti}:{no}: {satir.strip()}")

YOL.write_text(s, encoding="utf-8", newline="")
print(f"tamam: {len(adim)} düzenleme · {n0} -> {len(s)} bayt")
