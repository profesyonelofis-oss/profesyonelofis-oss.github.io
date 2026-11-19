import io

p = "src/main.rs"
s = io.open(p, encoding="utf-8", newline="").read().replace("\r\n", "\n")

# 1) LeadApp alanları
eski = "    /// Renklendirme paneli: demo renklerini kullanıcı seçer.\n    tema_panel_acik: bool,"
yeni = """    /// Renklendirme paneli: demo renklerini kullanıcı seçer.
    tema_panel_acik: bool,
    /// Hafıza paneli: serbest revizyon notu, kayıt listesi ve temizleme.
    hafiza_not: String,
    hafiza_panel: bool,
    hafiza_liste: Vec<ogrenme::Kayit>,
    hafiza_temizle_onay: bool,"""
assert eski in s, "alanlar"
s = s.replace(eski, yeni, 1)

# 2) Varsayılanlar
eski = "            tema_panel_acik: false,\n            tema_vurgu: ayar.demo_vurgu.clone(),"
yeni = """            tema_panel_acik: false,
            hafiza_not: String::new(),
            hafiza_panel: false,
            hafiza_liste: Vec::new(),
            hafiza_temizle_onay: false,
            tema_vurgu: ayar.demo_vurgu.clone(),"""
assert eski in s, "varsayilanlar"
s = s.replace(eski, yeni, 1)

# 3) Yardımcılar
eski = "fn demo_html(isletme: &BusinessLead, gonderen: &str) -> String {"
yeni = """/// Öğrenilen animasyon sınıfını sektör varsayılanına uygular.
fn ogrenilen_sinematik(varsayilan: bool, ders: &ogrenme::Ders) -> bool {
    match ders.animasyon.as_deref() {
        Some("kenburns+sinema") => true,
        Some("kenburns") => false,
        _ => varsayilan,
    }
}

/// Hafızaya yazılan animasyon sınıfı.
fn animasyon_sinifi(sinematik: bool) -> &'static str {
    if sinematik {
        "kenburns+sinema"
    } else {
        "kenburns"
    }
}

fn demo_html(isletme: &BusinessLead, gonderen: &str) -> String {"""
assert eski in s, "yardimcilar"
s = s.replace(eski, yeni, 1)

# 4) Üretimde öğrenilen animasyon
eski = """    let tasarim = sektor_tasarim::sec(&isletme.category);
    // Öğrenme hafızası: onaylanmış palet varsa sektör varsayılanını ezer.
    let ders = ogrenme::ders(&isletme.category);"""
yeni = """    let mut tasarim = sektor_tasarim::sec(&isletme.category);
    // Öğrenme hafızası: onaylanmış palet ve başarılı animasyon sınıfı
    // sektör varsayılanını ezer.
    let ders = ogrenme::ders(&isletme.category);
    tasarim.sinematik = ogrenilen_sinematik(tasarim.sinematik, &ders);"""
assert eski in s, "tasarim"
s = s.replace(eski, yeni, 1)

eski = """            if tasarim.sinematik {
                "kenburns+sinema"
            } else {
                "kenburns"
            },"""
yeni = """            animasyon_sinifi(tasarim.sinematik),"""
assert eski in s, "uretim animasyon"
s = s.replace(eski, yeni, 1)

# 5) Panel bloğu
bas = s.index("        ui.add_space(8.0);\n        let hafiza_sektor")
bit = s.index("        ui.add_space(14.0);", bas) + len("        ui.add_space(14.0);")
blok = '''        ui.add_space(8.0);
        let hafiza_sektor = self.sectors[self.selected_sector_idx];
        let hafiza_tasarim = sektor_tasarim::sec(hafiza_sektor);
        let ders = ogrenme::ders(hafiza_sektor);
        let hafiza_sinematik = ogrenilen_sinematik(hafiza_tasarim.sinematik, &ders);
        if ders.kayit_sayisi > 0 {
            let animasyon = ders.animasyon.as_deref().unwrap_or("kenburns");
            let not: String = ders
                .notlar
                .first()
                .map(|not| not.chars().take(60).collect())
                .unwrap_or_else(|| "onay bekliyor".to_string());
            ui.label(
                egui::RichText::new(format!(
                    "Hafıza · {animasyon} · {} kayıt · {not}",
                    ders.kayit_sayisi
                ))
                .size(9.5)
                .color(DIM),
            );
        }
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Revizyon notu").size(9.5).color(DIM));
        ui.add(
            egui::TextEdit::singleline(&mut self.hafiza_not)
                .hint_text("ör. zemin daha açık olsun")
                .desired_width(f32::INFINITY)
                .char_limit(170),
        );
        ui.add_space(4.0);
        if hayalet_buton(ui, "Paleti ve notu hafızaya al")
            .on_hover_text("Palet ve animasyon onaylanır; not sonraki demolarda ders olur")
            .clicked()
        {
            let vurgu = if self.tema_vurgu.trim().is_empty() {
                hafiza_tasarim.vurgu.to_string()
            } else {
                self.tema_vurgu.clone()
            };
            let zemin = if self.tema_zemin.trim().is_empty() {
                hafiza_tasarim.zemin.to_string()
            } else {
                self.tema_zemin.clone()
            };
            let yazi = if self.tema_yazi.trim().is_empty() {
                hafiza_tasarim.yazi.to_string()
            } else {
                self.tema_yazi.clone()
            };
            ogrenme::onay(
                hafiza_sektor,
                hafiza_tasarim.grup,
                animasyon_sinifi(hafiza_sinematik),
                &vurgu,
                &zemin,
                &yazi,
                &self.hafiza_not,
            );
            let not_var = !self.hafiza_not.trim().is_empty();
            self.hafiza_not.clear();
            self.logs.push(if not_var {
                format!("[i] Palet ve not hafızaya alındı · {hafiza_sektor}")
            } else {
                format!("[i] Palet hafızaya alındı · {hafiza_sektor}")
            });
            if self.hafiza_panel {
                self.hafiza_liste = ogrenme::kayitlar();
            }
        }
        ui.add_space(8.0);

        // Hafıza yönetimi: kayıtları gör, tek tek sil veya tümünü temizle.
        ui.horizontal(|ui| {
            let etiket = if self.hafiza_panel {
                "Hafızayı kapat"
            } else {
                "Hafızayı görüntüle / temizle"
            };
            if hayalet_buton(ui, etiket).clicked() {
                self.hafiza_panel = !self.hafiza_panel;
                self.hafiza_temizle_onay = false;
                if self.hafiza_panel {
                    self.hafiza_liste = ogrenme::kayitlar();
                }
            }
            if self.hafiza_panel && hayalet_buton(ui, "Yenile").clicked() {
                self.hafiza_liste = ogrenme::kayitlar();
            }
        });

        if self.hafiza_panel {
            ui.add_space(4.0);
            if self.hafiza_liste.is_empty() {
                ui.label(
                    egui::RichText::new("Kayıt yok. İlk onaydan sonra burada görünür.")
                        .size(10.0)
                        .color(DIM),
                );
            } else {
                let mut silinecek: Option<usize> = None;
                egui::ScrollArea::vertical()
                    .id_source("hafiza_listesi")
                    .max_height(190.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (sira, kayit) in self.hafiza_liste.iter().enumerate() {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} · {} · {}×",
                                        kayit.sektor, kayit.animasyon, kayit.sayi
                                    ))
                                    .size(10.0)
                                    .color(METIN),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} / {} / {}",
                                        kayit.palet.vurgu, kayit.palet.zemin, kayit.palet.yazi
                                    ))
                                    .size(9.0)
                                    .color(SOLUK),
                                );
                                if hayalet_buton(ui, "Sil").clicked() {
                                    silinecek = Some(sira);
                                }
                            });
                            if !kayit.not.trim().is_empty() {
                                let not: String = kayit.not.chars().take(90).collect();
                                ui.label(
                                    egui::RichText::new(format!("“{not}”")).size(9.5).color(DIM),
                                );
                            }
                            ui.add_space(4.0);
                        }
                    });
                if let Some(sira) = silinecek {
                    let ad = self
                        .hafiza_liste
                        .get(sira)
                        .map(|kayit| kayit.sektor.clone())
                        .unwrap_or_default();
                    if ogrenme::sil(sira) {
                        self.hafiza_liste = ogrenme::kayitlar();
                        self.logs.push(format!("[i] Hafıza kaydı silindi · {ad}"));
                    }
                }
            }
            ui.add_space(6.0);
            if self.hafiza_temizle_onay {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("Tüm kayıtlar silinsin mi?")
                            .size(10.0)
                            .color(VURGU),
                    );
                    if hayalet_buton(ui, "Evet, temizle").clicked() {
                        ogrenme::temizle();
                        self.hafiza_liste.clear();
                        self.hafiza_temizle_onay = false;
                        self.logs.push("[i] Hafıza temizlendi".to_string());
                    }
                    if hayalet_buton(ui, "Vazgeç").clicked() {
                        self.hafiza_temizle_onay = false;
                    }
                });
            } else if hayalet_buton(ui, "Tüm hafızayı temizle").clicked() {
                self.hafiza_temizle_onay = true;
            }
        }
        ui.add_space(14.0);'''
s = s[:bas] + blok + s[bit:]

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("main.rs yamalandi")
