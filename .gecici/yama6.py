import re
import sys

def oku(p):
    return open(p, encoding='utf-8').read()

def yaz(p, s):
    open(p, 'w', encoding='utf-8', newline='\n').write(s)

def degistir(p, eski, yeni, ad):
    s = oku(p)
    if eski not in s:
        sys.exit("BULUNAMADI: " + ad)
    yaz(p, s.replace(eski, yeni, 1))
    print("degisti:", ad)

# ---------------------------------------------------------------- main.rs
degistir('src/main.rs', "mod profil;", "mod profil;\nmod harita;\nmod teshis;", "main.rs moduller")
degistir(
    'src/main.rs',
    "    /// [2]/[3] için Local SEO denetimi (kategori, NAP, koordinat, ...).\n    seo_denetim: Option<crate::profil::Denetim>,",
    '''    /// [2]/[3] için Local SEO denetimi (kategori, NAP, koordinat, ...).
    seo_denetim: Option<crate::profil::Denetim>,
    /// Sağdaki gerçek OpenStreetMap haritası (karo önbelleği + pin).
    harita: crate::harita::Harita,
    /// [3] SSR teşhisi: (slug, rapor) — aynı sayfa için tekrar çekilmez.
    teshis: Option<(String, crate::teshis::Rapor)>,''',
    "main.rs alanlar",
)
degistir(
    'src/main.rs',
    "            seo_denetim: None,",
    "            seo_denetim: None,\n            harita: crate::harita::Harita::default(),\n            teshis: None,",
    "main.rs init",
)

# ------------------------------------------------------------- katmanlar.rs
p = 'src/katmanlar.rs'

# 1) [2]: temsili ızgara yerine sağda gerçek harita + solda dikey panel
degistir(
    p,
    '''                self.seo_linki_otomatik(ctx);
                ui.add_space(12.0);
                match self.seo_konum.clone() {
                    Some(konum) => {
                        let boyut = egui::vec2(ui.available_width(), 300.0);
                        let (rect, _) = ui.allocate_exact_size(boyut, egui::Sense::hover());
                        harita_ciz(ui, rect, &konum, true);
                        ui.add_space(6.0);
                        let place = if konum.place_id.is_empty() {
                            "—"
                        } else {
                            konum.place_id.as_str()
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} · place_id: {place}",
                                konum.ad,
                                konum.koordinat()
                            ))
                            .size(10.0)
                            .color(METIN),
                        );
                    }
                    None => {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("Harita: henüz konum çözülmedi.")
                                    .size(11.0)
                                    .color(DIM),
                            );
                        });
                    }
                }''',
    '''                self.seo_linki_otomatik(ctx);
                ui.add_space(12.0);
                // Sağda sabit harita paneli (gerçek OSM karoları), solda dikey
                // SEO çalışma listesi. Harita paneli yeniden boyutlandırılabilir.
                let harita_gen = (ui.available_width() * 0.44).clamp(300.0, 820.0);
                let konum = self.seo_konum.clone();
                egui::SidePanel::right("seo2_harita_paneli")
                    .resizable(true)
                    .default_width(harita_gen)
                    .min_width(260.0)
                    .max_width(900.0)
                    .frame(
                        egui::Frame::none()
                            .fill(ARKA_PLAN)
                            .inner_margin(egui::Margin::same(6.0)),
                    )
                    .show_inside(ui, |ui| {
                        let genislik = ui.available_width();
                        let yukseklik = (genislik * 0.92)
                            .min(ui.available_height() - 52.0)
                            .max(200.0);
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(genislik, yukseklik),
                            egui::Sense::hover(),
                        );
                        let pin = konum
                            .as_ref()
                            .map(|k| (k.enlem, k.boylam, k.ad.clone()));
                        self.harita.ciz(ui, rect, pin.as_ref(), true);
                        ui.add_space(6.0);
                        match konum.as_ref() {
                            Some(k) => {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} · {} · place_id: {}",
                                        k.ad,
                                        k.koordinat(),
                                        if k.place_id.is_empty() {
                                            "—"
                                        } else {
                                            k.place_id.as_str()
                                        }
                                    ))
                                    .size(9.5)
                                    .color(METIN),
                                );
                            }
                            None => {
                                ui.label(
                                    egui::RichText::new(
                                        "Harita: link yapıştırınca pin burada işaretlenir.",
                                    )
                                    .size(10.0)
                                    .color(DIM),
                                );
                            }
                        }
                    });
                egui::ScrollArea::vertical()
                    .id_source("seo2_sol_panel")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {''',
    "katmanlar [2] harita paneli",
)

# [2] kapanış: denetim bloğundan sonra ScrollArea kapat
degistir(
    p,
    '''                    for oneri in denetim.oneriler() {
                        ui.label(
                            egui::RichText::new(format!("→ {oneri}"))
                                .size(9.5)
                                .color(SOLUK),
                        );
                    }
                }
            });
    }

    /// Katman 3''',
    '''                    for oneri in denetim.oneriler() {
                        ui.label(
                            egui::RichText::new(format!("→ {oneri}"))
                                .size(9.5)
                                .color(SOLUK),
                        );
                    }
                }
                    });
            });
    }

    /// Katman 3''',
    "katmanlar [2] scroll kapanisi",
)

# 2) [3]: aynı iki panelli düzen
degistir(
    p,
    '''                if let Some(konum) = self.seo_konum.clone() {
                    let boyut = egui::vec2(ui.available_width(), 130.0);
                    let (rect, _) = ui.allocate_exact_size(boyut, egui::Sense::hover());
                    harita_ciz(ui, rect, &konum, false);
                    ui.add_space(8.0);
                }''',
    '''                // Sağda harita pini paneli, solda dikey teşhis listesi.
                let harita_gen = (ui.available_width() * 0.44).clamp(300.0, 820.0);
                let konum3 = self.seo_konum.clone();
                egui::SidePanel::right("seo3_harita_paneli")
                    .resizable(true)
                    .default_width(harita_gen)
                    .min_width(260.0)
                    .max_width(900.0)
                    .frame(
                        egui::Frame::none()
                            .fill(ARKA_PLAN)
                            .inner_margin(egui::Margin::same(6.0)),
                    )
                    .show_inside(ui, |ui| {
                        let genislik = ui.available_width();
                        let yukseklik = (genislik * 0.92)
                            .min(ui.available_height() - 52.0)
                            .max(200.0);
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(genislik, yukseklik),
                            egui::Sense::hover(),
                        );
                        let pin = konum3
                            .as_ref()
                            .map(|k| (k.enlem, k.boylam, k.ad.clone()));
                        self.harita.ciz(ui, rect, pin.as_ref(), false);
                        ui.add_space(6.0);
                        match konum3.as_ref() {
                            Some(k) => {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "PİN · {} · {}",
                                        k.ad,
                                        k.koordinat()
                                    ))
                                    .size(9.5)
                                    .color(METIN),
                                );
                            }
                            None => {
                                ui.label(
                                    egui::RichText::new(
                                        "Pin yok: 2. sekmede linki çözün.",
                                    )
                                    .size(10.0)
                                    .color(DIM),
                                );
                            }
                        }
                    });
                egui::ScrollArea::vertical()
                    .id_source("seo3_sol_panel")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {''',
    "katmanlar [3] harita paneli",
)

# [3] kapanış: onay satırından sonra ScrollArea kapat
degistir(
    p,
    '''                    ui.label(
                        egui::RichText::new(format!("onaylanan: {}", self.seo_onaylanan.len()))
                            .size(10.0)
                            .color(DIM),
                    );
                });
            });
    }
}''',
    '''                    ui.label(
                        egui::RichText::new(format!("onaylanan: {}", self.seo_onaylanan.len()))
                            .size(10.0)
                            .color(DIM),
                    );
                });
                    });
            });
    }
}''',
    "katmanlar [3] scroll kapanisi",
)

# 3) [3]: etiket sayımı yerine yeşil/kırmızı teşhis listesi
degistir(
    p,
    '''                ui.add_space(6.0);
                let (durum, etiketler) = ssr_ozet(&slug);
                ui.label(
                    egui::RichText::new(format!("SSR durumu: {durum}"))
                        .size(10.0)
                        .color(MAVI),
                );
                let gorselsiz = medya == 0;
                for satir in &etiketler {
                    // Medya etiketleri yalnız medyası olan sayfalarda çıkar:
                    // yokluğu hata değil, işletmenin fotoğrafı/videosu yok demek.
                    let medya_etiketi = satir.starts_with("autoplay")
                        || satir.starts_with("loading=\\"lazy\\"")
                        || satir.starts_with("property=\\"og:image\\"");
                    let renk = if !satir.ends_with("× 0") {
                        YESIL
                    } else if medya_etiketi && gorselsiz {
                        SOLUK
                    } else {
                        KIRMIZI
                    };
                    ui.label(egui::RichText::new(satir).size(9.0).monospace().color(renk));
                }
                if gorselsiz {''',
    '''                ui.add_space(6.0);
                let gorselsiz = medya == 0;
                let pin = self.seo_konum.as_ref().map(|k| (k.enlem, k.boylam));
                let video_var = site
                    .get("medya")
                    .and_then(|m| m.as_array())
                    .map(|liste| {
                        liste.iter().any(|kayit| {
                            kayit.get("tur").and_then(|t| t.as_str()) == Some("video")
                        })
                    })
                    .unwrap_or(false);
                let taze = match &self.teshis {
                    Some((anahtar, _)) => anahtar != &slug,
                    None => true,
                };
                if taze {
                    self.teshis = Some((
                        slug.clone(),
                        crate::teshis::sunucudan(&slug, pin, medya, video_var),
                    ));
                }
                if let Some((_, rapor)) = self.teshis.clone() {
                    ui.label(
                        egui::RichText::new(format!("SSR durumu: {}", rapor.durum))
                            .size(10.0)
                            .color(if rapor.durum.contains("200") { YESIL } else { KIRMIZI }),
                    );
                    ui.label(
                        egui::RichText::new(rapor.ozet())
                            .size(9.5)
                            .color(if rapor.hepsi_gecti() { YESIL } else { KEHRIBAR }),
                    );
                    for madde in rapor.maddeler.iter() {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("●")
                                    .size(9.0)
                                    .color(if madde.gecti { YESIL } else { KIRMIZI }),
                            );
                            ui.label(
                                egui::RichText::new(format!("{}: {}", madde.etiket, madde.deger))
                                    .size(9.5)
                                    .color(if madde.gecti { METIN } else { KIRMIZI }),
                            );
                        });
                        if !madde.gecti && !madde.not.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("   {}", madde.not))
                                    .size(8.5)
                                    .color(SOLUK),
                            );
                        }
                    }
                    ui.add_space(4.0);
                    if ikincil_buton(ui, "TEŞHİSİ YENİLE", true, Some(egui::vec2(150.0, 26.0)))
                        .clicked()
                    {
                        self.teshis = None;
                    }
                }
                if gorselsiz {''',
    "katmanlar [3] teshis listesi",
)

# 4) eski harita/ssr_ozet fonksiyonlarını kaldır
s = oku(p)
desen = re.compile(
    r"/// Koordinatı harita ızgarası üzerinde işaretler.*?(?=impl LeadApp \{\n    /// \[2\]'de çözülen)",
    re.S,
)
yeni, sayi = desen.subn("", s)
assert sayi == 1, f"harita_ciz/ssr_ozet silinemedi ({sayi})"
yaz(p, yeni)
print("degisti: eski harita_ciz ve ssr_ozet kaldirildi")

# 5) yeni çözümde haritayı odakla ve teşhisi geçersiz kıl
degistir(
    p,
    '''        self.seo_durum = format!("Profil denetimi: {}", denetim.ozet());
        self.seo_denetim = Some(denetim);
        self.seo_kayit = isletme.kayit.clone();
        self.seo_konum = Some(isletme.konum);''',
    '''        self.seo_durum = format!("Profil denetimi: {}", denetim.ozet());
        self.seo_denetim = Some(denetim);
        self.seo_kayit = isletme.kayit.clone();
        self.harita.odakla(isletme.konum.enlem, isletme.konum.boylam);
        self.teshis = None;
        self.seo_konum = Some(isletme.konum);''',
    "katmanlar harita odak",
)

# ---------------------------------------------------------------- sunucu.rs
p = 'src/bin/sunucu.rs'
degistir(
    p,
    '''    let jsonld = serde_json::to_string(&jsonld).unwrap_or_else(|_| "{}".to_string());''',
    '''    // Yerel on-page sinyali: hizmet + ilçe/il başlıkta; CTR için yol tarifi
    // ve tıkla-ara düğmeleri (harita piniyle aynı koordinat).
    let h2 = match (site.il.trim().is_empty(), site.ilce.trim().is_empty()) {
        (true, true) => format!("{} hizmeti", site.sektor),
        (true, false) => format!("{} hizmeti · {}", site.sektor, site.ilce),
        (false, true) => format!("{} hizmeti · {}", site.sektor, site.il),
        (false, false) => format!("{} hizmeti · {}, {}", site.sektor, site.ilce, site.il),
    };
    let mut cta = String::new();
    if let (Some(enlem), Some(boylam)) = (site.enlem, site.boylam) {
        cta.push_str(&format!(
            "<a class=\\"cta\\" href=\\"https://www.google.com/maps/dir/?api=1&destination={enlem:.6},{boylam:.6}\\" rel=\\"noopener\\">Yol tarifi al</a>"
        ));
    }
    if !site.telefon.is_empty() {
        let rakam: String = site
            .telefon
            .chars()
            .filter(|k| k.is_ascii_digit() || *k == '+')
            .collect();
        if rakam.len() >= 10 {
            cta.push_str(&format!(
                "<a class=\\"cta ikincil\\" href=\\"tel:{rakam}\\">Hemen ara</a>"
            ));
        }
    }
    let jsonld = serde_json::to_string(&jsonld).unwrap_or_else(|_| "{}".to_string());''',
    "sunucu h2 + cta",
)
degistir(
    p,
    '''.telefon{{display:inline-block;margin-top:12px;padding:12px 20px;border-radius:10px;background:var(--vurgu);color:#fff;text-decoration:none;font-weight:600}}''',
    '''.telefon{{display:inline-block;margin-top:12px;padding:12px 20px;border-radius:10px;background:var(--vurgu);color:#fff;text-decoration:none;font-weight:600}}
h2{{font-size:18px;font-weight:500;color:var(--soluk);margin:0 0 10px}}
.cta-satir{{display:flex;gap:10px;flex-wrap:wrap;margin:16px 0 8px}}
.cta{{padding:11px 20px;border-radius:10px;background:var(--vurgu);color:#fff;text-decoration:none;font-weight:600}}
.cta.ikincil{{background:#21262d;border:1px solid #30363d;color:var(--yazi)}}''',
    "sunucu css",
)
degistir(
    p,
    '''<h1>{ad}</h1>
<div class="ust"><span>{sektor}</span><span>{yer}</span>{telefon}</div>''',
    '''<h1>{ad}</h1>
<h2>{h2}</h2>
<div class="ust"><span>{sektor}</span><span>{yer}</span>{telefon}</div>
<div class="cta-satir">{cta}</div>''',
    "sunucu html h2/cta",
)
degistir(
    p,
    '''        ad = kacis(&site.ad),
        yer = kacis(&yer),''',
    '''        ad = kacis(&site.ad),
        h2 = kacis(&h2),
        cta = cta,
        yer = kacis(&yer),''',
    "sunucu format argumanlari",
)
degistir(
    p,
    '''        assert_eq!(deger["openingHours"], "Mo-Su 10:00-23:00");
    }''',
    '''        assert_eq!(deger["openingHours"], "Mo-Su 10:00-23:00");
    }

    #[test]
    fn yerel_baslik_ve_ctr_dugmeleri_uretilir() {
        let site = ornek();
        let html = sayfa(&site, "https://ornek.com");
        assert!(html.contains("<h2>Restoran hizmeti · Manisa</h2>"));
        assert!(html.contains("Yol tarifi al"));
        assert!(html.contains("href=\\"tel:05551112233\\""));
        assert!(html.contains("destination=38.612300,27.432100"));
        // Telefonsuz işletmede ara düğmesi çıkmaz, yol tarifi kalır.
        let sessiz = Site {
            telefon: String::new(),
            ..ornek()
        };
        let html2 = sayfa(&sessiz, "https://ornek.com");
        assert!(!html2.contains("Hemen ara"));
        assert!(html2.contains("Yol tarifi al"));
    }''',
    "sunucu cta testi",
)
print("TAMAM")
