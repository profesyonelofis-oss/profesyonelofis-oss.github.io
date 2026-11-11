"""Keşif ekranını sabit 50/50 iki sütuna alır; parametre bloğu üstte sabit,
sonuç listesi altta iç kaydırmalı, sağ yarı harita 100% × 100%.

Yama metin çapalıdır: her adım bulunamazsa betik durur, dosya yarım kalmaz.
"""
import sys

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

YOL = "src/main.rs"
metin = open(YOL, encoding="utf-8").read()
adimlar = []


def degistir(eski, yeni, ad):
    global metin
    sayi = metin.count(eski)
    assert sayi == 1, f"{ad}: {sayi} eşleşme"
    metin = metin.replace(eski, yeni, 1)
    adimlar.append(ad)


def kes(bas, son):
    global metin
    i = metin.find(bas)
    assert i >= 0, f"başlangıç yok: {bas[:60]}"
    j = metin.find(son, i)
    assert j > i, f"bitiş yok: {son[:60]}"
    metin = metin[:i] + metin[j + len(son):]


# 1) Sol panel: %20 filtre paneli yerine ekranın yarısı olan keşif sütunu.
degistir(
    """        // --- Sol panel: filtreler -------------------------------------------
        egui::SidePanel::left("filtre_paneli")
            .resizable(false)
            .exact_width((ctx.screen_rect().width() * 0.20).clamp(260.0, 340.0))""",
    """        // --- Sol sütun: parametreler (üst, sabit) + sonuçlar (alt, kayar) ---
        // Ekran iki eşit sütuna bölünür: sol %50 arama/sonuç, sağ %50 harita.
        // Dış panelin yüksekliği sabittir; yalnız sonuç listesi içte kayar.
        let sol_genislik = (ctx.screen_rect().width() * 0.5).max(440.0);
        egui::SidePanel::left("kesif_sol")
            .resizable(false)
            .exact_width(sol_genislik)""",
    "sol panel kimliği ve genişliği",
)

# 2) Panel başlığı: keşif başlığı + PARAMETRELER.
degistir(
    """                ui.horizontal(|ui| {
                    bolum_etiketi(ui, "FİLTRELER");""",
    """                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("Keşif & Radar")
                                .font(baslik_font(17.0))
                                .color(METIN),
                        );
                        let yer = if self.il_geneli {
                            format!("{il} · tüm ilçeler · {sektor}")
                        } else {
                            format!("{il} · {ilce} · {sektor}")
                        };
                        ui.label(egui::RichText::new(yer).size(10.5).color(DIM));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        kontur_pill(ui, "Places API (New)");
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    bolum_etiketi(ui, "PARAMETRELER");""",
    "keşif başlığı ve parametre başlığı",
)

# 3) Kullanılmayan "Sektör verimi" günlük düğmesi kaldırılır; parametre
#    bloğu sabit yükseklikli bir alana alınır.
degistir(
    """                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} il · {} ilçe · {} sektör",
                            iller::ILLER.len(),
                            iller::ILCELER,
                            self.sectors.len()
                        ))
                        .size(10.5)
                        .color(DIM),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if hayalet_buton(ui, "Sektör verimi")
                            .on_hover_text(
                                "Önbellekteki taramalara göre hangi sektörün kayıt döndürdüğünü günlüğe yazar",
                            )
                            .clicked()
                        {
                            self.verim_raporunu_yaz();
                        }
                    });
                });
                ui.add_space(10.0);""",
    """                ui.label(
                    egui::RichText::new(format!(
                        "{} il · {} ilçe · {} sektör",
                        iller::ILLER.len(),
                        iller::ILCELER,
                        self.sectors.len()
                    ))
                    .size(10.5)
                    .color(DIM),
                );
                ui.add_space(8.0);

                // Parametre bloğu: üstte sabit yükseklik, kendi içinde kayar.
                let param_yukseklik = (ui.available_height() * 0.52).clamp(250.0, 430.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), param_yukseklik),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {""",
    "parametre bloğu açılışı",
)

# 4) Parametre bloğu kapanışı + sonuç bölümü.
degistir(
    """                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(if self.il_geneli {
                            "Tüm ilçeler sırayla · en fazla 20 kayıt"
                        } else {
                            "Sorgu başına en fazla 20 kayıt"
                        })
                        .size(10.0)
                        .color(DIM),
                    );
                });
            });""",
    """                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(if self.il_geneli {
                            "Tüm ilçeler sırayla · en fazla 20 kayıt"
                        } else {
                            "Sorgu başına en fazla 20 kayıt"
                        })
                        .size(10.0)
                        .color(DIM),
                    );
                });
                    },
                );

                // --- Sonuç listesi: alan sabit, liste kendi içinde kayar -----
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    bolum_etiketi(ui, "ARAMA SONUÇLARI");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{} sonuç", self.leads.len()))
                                .size(10.0)
                                .color(DIM),
                        );
                    });
                });
                ui.add_space(2.0);
                self.kesif_sonuclari(ui);
            });""",
    "parametre bloğu kapanışı ve sonuç bölümü",
)

# 5) Parametre bloğu içindeki ölçüler küçültülür (taşma olmasın).
degistir(
    """                let wa_yuksekligi = if self.whatsapp_acik { 252.0 } else { 104.0 };
                let kaydirma_yuksekligi = (ui.available_height() - wa_yuksekligi - 96.0).max(140.0);""",
    """                let wa_yuksekligi = if self.whatsapp_acik { 206.0 } else { 66.0 };
                let kaydirma_yuksekligi = (ui.available_height() - wa_yuksekligi - 76.0).max(96.0);""",
    "parametre kaydırma yüksekliği",
)
degistir('secim_listesi(ui, "il_listesi", 104.0, None, |ui| {',
         'secim_listesi(ui, "il_listesi", 70.0, None, |ui| {',
         "il listesi yüksekliği")
degistir('secim_listesi(ui, "ilce_listesi", 130.0, ofset, |ui| {',
         'secim_listesi(ui, "ilce_listesi", 88.0, ofset, |ui| {',
         "ilçe listesi yüksekliği")

# 6) İşlem günlüğü: sabit yükseklik, taşma yok.
degistir(
    """        egui::TopBottomPanel::bottom("gunluk_paneli")
            .resizable(true)
            .default_height(100.0)
            .min_height(84.0)
            .max_height(280.0)""",
    """        egui::TopBottomPanel::bottom("gunluk_paneli")
            .resizable(false)
            .exact_height(76.0)""",
    "günlük paneli sabit yükseklik",
)
degistir("                let gunluk_yuksekligi = (ui.available_height() - 18.0).max(40.0);",
         "                let gunluk_yuksekligi = (ui.available_height() - 4.0).max(24.0);",
         "günlük kaydırma yüksekliği")

# 7) Boş sonuç ekranındaki yer tutucu adım kartları kaldırılır.
i = metin.find("        ui.add_space(20.0);\n        let adim_genisligi = 196.0;")
assert i >= 0, "adım kartı bloğu yok"
j = metin.find("        });\n    });\n}", i)
assert j > i, "adım kartı bloğu bitişi yok"
metin = metin[:i] + "    });\n}" + metin[j + len("        });\n    });\n}"):]
adimlar.append("adım kartı bloğu")

# 8) Kullanılmayan yardımcılar: adim_karti ve verim_raporunu_yaz.
for baslik, bitis in [
    ("fn adim_karti(ui: &mut egui::Ui, no: &str, baslik: &str, aciklama: &str, genislik: f32) {",
     "\n}\n"),
    ("    fn verim_raporunu_yaz(&mut self) {", "\n    }\n"),
]:
    i = metin.find(baslik)
    assert i >= 0, f"fonksiyon yok: {baslik[:40]}"
    j = metin.find(bitis, i)
    assert j > i, f"fonksiyon bitişi yok: {baslik[:40]}"
    metin = metin[:i] + metin[j + len(bitis):]
adimlar.append("kullanılmayan yardımcılar")

# 9) Sonuç kartı: harita vurgusu için kalın kenar ve kart dikdörtgeni.
degistir(
    """    limit_doldu: bool,
) -> (bool, bool) {""",
    """    limit_doldu: bool,
    vurgu: bool,
) -> (bool, bool, egui::Rect) {""",
    "kart_ciz imzası",
)
degistir(
    """    let kenar = if acik {
        ton(MAVI, 0.55)
    } else if secili {""",
    """    let kenar = if acik || vurgu {
        ton(MAVI, 0.80)
    } else if secili {""",
    "kart kenar vurgusu",
)
degistir(
    """        .fill(if secili || acik { KART_SECILI } else { KART })
        .stroke(egui::Stroke::new(1.0_f32, kenar))""",
    """        .fill(if secili || acik || vurgu { KART_SECILI } else { KART })
        .stroke(egui::Stroke::new(
            if vurgu { 2.0_f32 } else { 1.0_f32 },
            kenar,
        ))""",
    "kart dolgusu ve kenar kalınlığı",
)
degistir("    (detay_iste, onizleme_iste)\n}",
         "    (detay_iste, onizleme_iste, ic.response.rect)\n}",
         "kart_ciz dönüşü")

# 10) Harita tıklaması listede vurgulanacak kartı belirler.
degistir(
    """    harita_imza: (usize, Option<String>),""",
    """    harita_imza: (usize, Option<String>),
    /// Haritadan pin seçildiğinde listede kaydırılıp vurgulanacak kart.
    vurgu_kart: Option<usize>,""",
    "vurgu_kart alanı",
)
degistir("            harita_imza: (0, None),",
         "            harita_imza: (0, None),\n            vurgu_kart: None,",
         "vurgu_kart başlangıcı")

# 11) Kart çizimi: vurgu bayrağı ve kaydırma.
degistir(
    """                                let (detay_iste, wa_iste) = kart_ciz(
                                    ui,
                                    lead,
                                    &gonderen,
                                    logs,
                                    acik_detay == Some(idx),
                                    limit_doldu,
                                );""",
    """                                let (detay_iste, wa_iste, kart_rect) = kart_ciz(
                                    ui,
                                    lead,
                                    &gonderen,
                                    logs,
                                    acik_detay == Some(idx),
                                    limit_doldu,
                                    vurgu == Some(idx),
                                );
                                // Haritadaki pin seçildiyse kart görünür alana gelir.
                                if vurgu == Some(idx) {
                                    ui.scroll_to_rect(kart_rect, Some(egui::Align::Center));
                                }""",
    "kart çizimi çağrısı",
)
degistir(
    """                    let acik_detay = self.detay_idx;
                    let leads = &mut self.leads;""",
    """                    let acik_detay = self.detay_idx;
                    // Vurgu tek kareliktir: tüketilmezse her karede kaydırır.
                    let vurgu = self.vurgu_kart;
                    self.vurgu_kart = None;
                    let leads = &mut self.leads;""",
    "vurgu tüketimi",
)

# 12) Kullanılmayan demo sayacı (istatistik kartlarıyla birlikte kalktı).
degistir(
    """        let demo_hazir = self
            .leads
            .iter()
            .filter(|lead| lead.demo_yolu.is_some())
            .count();
""",
    "",
    "demo_hazir sayacı",
)

# 13) Sağ yarı + sonuç bloğunun taşınması.
m = metin.find("        // --- Sağ sütun: canlı harita")
assert m > 0, "harita bloğu yok"
s = metin.find("        // --- Orta panel: sonuçlar", m)
assert s > m, "sonuç bloğu yok"
govde_bas = metin.find("            .show(ctx, |ui| {\n", s)
assert govde_bas > s, "sonuç gövdesi yok"
govde_bas += len("            .show(ctx, |ui| {\n")
bitis = metin.find("\n            });\n    }\n}", govde_bas)
assert bitis > govde_bas, "sonuç bloğu kapanışı yok"
govde = metin[govde_bas:bitis]

# Gövdenin başındaki başlık/istatistik/demo satırı kaldırılır.
t = govde.find("                // Araç çubuğu: sonuç araması solda, işlemler sağda.")
assert t > 0, "araç çubuğu yok"
govde = govde[t:]
govde = "\n".join(satir[8:] if satir.startswith("        ") else satir
                  for satir in govde.split("\n"))

harita = """        // --- Sağ yarı: canlı harita -----------------------------------------
        // Ekranın tam sağ yarısı: harita paneli 100% × 100% doldurur ve sol
        // sütunla aynı yüksekliktedir; arada boşluk kalmaz.
        let mut harita_tiklama: Option<usize> = None;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ARKA_PLAN)
                    .inner_margin(egui::Margin::symmetric(10.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    bolum_etiketi(ui, "CANLI HARİTA");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (nokta, _) =
                            ui.allocate_exact_size(egui::vec2(7.0, 7.0), egui::Sense::hover());
                        ui.painter().circle_filled(nokta.center(), 3.0, YESIL);
                        ui.label(
                            egui::RichText::new(format!(
                                "Google Places · {} katman",
                                self.harita.stil().etiket()
                            ))
                            .size(9.5)
                            .color(DIM),
                        );
                    });
                });
                ui.add_space(4.0);
                let yukseklik = ui.available_height().max(200.0);
                harita_tiklama = self.kesif_haritasi(ui, yukseklik);
            });
        // Haritadaki pin seçimi sol listedeki kartı vurgular.
        if let Some(idx) = harita_tiklama {
            self.detay_idx = Some(idx);
            self.onizleme_lead = Some(idx);
            self.vurgu_kart = Some(idx);
        }
    }

"""

yontem = """    /// Keşif ekranının alt bölümü: arama sonuçları listesi.
    ///
    /// Liste kendi içinde dikey kaydırılır; dış panelin yüksekliği değişmez.
    pub(crate) fn kesif_sonuclari(&mut self, ui: &mut egui::Ui) {
        let gonderen = whatsapp_numarasi(&self.whatsapp_phone)
            .map(|numara| format!("+{numara}"))
            .unwrap_or_default();
        let sektor = self.sectors[self.selected_sector_idx];
        let secili = self.leads.iter().filter(|lead| lead.selected).count();
""" + govde + """    }
"""

kalan = metin[bitis + len("\n            });\n    }\n}"):]
metin = metin[:m] + harita + yontem + "}" + kalan
adimlar.append("harita ve sonuç listesi yerleşimi")

open(YOL, "w", encoding="utf-8", newline="\n").write(metin)
print("uygulandı:", ", ".join(adimlar))
