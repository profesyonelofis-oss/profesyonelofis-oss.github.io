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

# ---------------------------------------------------------------------------
# 1) isletme_linki.rs — Isletme (konum + Places profili)
# ---------------------------------------------------------------------------
degistir(
    'src/isletme_linki.rs',
    '#[cfg(test)]\nmod testler {',
    '''/// Linkten çözülen işletme: harita pini ve (okunabildiyse) Google profili.
pub struct Isletme {
    pub konum: Konum,
    pub kayit: Option<crate::google::BizKayit>,
}

/// Arayüz yolu: konumu çözer, ardından harita profilini (adres, telefon,
/// saatler, yıldız, yorum) tek Places isteğiyle okur. Profil okunamazsa
/// (kota/ağ) yalnız konum döner; akış durmaz.
pub fn isletme_coz(link: &str) -> Result<Isletme, String> {
    let mut konum = konum_coz(link)?;
    let kayit = crate::google::ada_gore(&konum.ad).ok();
    if let Some(kayit) = &kayit {
        if konum.place_id.is_empty() {
            konum.place_id = place_id_bul(&kayit.kaynak);
        }
    }
    Ok(Isletme { konum, kayit })
}

#[cfg(test)]
mod testler {''',
    "isletme_linki Isletme + isletme_coz",
)

# ---------------------------------------------------------------------------
# 2) web_veri.rs — profil alanları sayfaya işlenir
# ---------------------------------------------------------------------------
degistir(
    'src/web_veri.rs',
    '            "medya": medya,',
    '''            "saatler": lead.calisma_saatleri,
            "enlem": lead.konum.map(|(enlem, _)| enlem),
            "boylam": lead.konum.map(|(_, boylam)| boylam),
            "yildiz": lead.yildiz_puani,
            "yorum": lead.inceleme_sayisi,
            "medya": medya,''',
    "web_veri profil alanlari",
)

# ---------------------------------------------------------------------------
# 3) sunucu.rs — Site alanları + JSON-LD geo/openingHours/rating
# ---------------------------------------------------------------------------
degistir(
    'src/bin/sunucu.rs',
    "    pub sema: String,\n    pub medya: Vec<Medya>,\n}",
    '''    pub sema: String,
    /// Harita pini: JSON-LD `geo` (GeoCoordinates) alanı.
    pub enlem: Option<f64>,
    pub boylam: Option<f64>,
    /// Google profilinden gelen çalışma saatleri (boş olabilir).
    pub saatler: String,
    pub yildiz: Option<f32>,
    pub yorum: Option<u32>,
    pub medya: Vec<Medya>,
}''',
    "sunucu Site alanlari",
)
degistir(
    'src/bin/sunucu.rs',
    '                sema: metin("sema"),\n                medya,',
    '''                sema: metin("sema"),
                enlem: site.get("enlem").and_then(|v| v.as_f64()),
                boylam: site.get("boylam").and_then(|v| v.as_f64()),
                saatler: metin("saatler"),
                yildiz: site.get("yildiz").and_then(|v| v.as_f64()).map(|v| v as f32),
                yorum: site
                    .get("yorum")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32),
                medya,''',
    "sunucu Veri::json alanlari",
)
degistir(
    'src/bin/sunucu.rs',
    '''    if !gorsel.is_empty() {
        jsonld["image"] = serde_json::Value::String(gorsel.clone());
    }''',
    '''    if !gorsel.is_empty() {
        jsonld["image"] = serde_json::Value::String(gorsel.clone());
    }
    // Harita pini ile web sayfası aynı koordinatı göstersin: GeoCoordinates.
    if let (Some(enlem), Some(boylam)) = (site.enlem, site.boylam) {
        jsonld["geo"] = serde_json::json!({
            "@type": "GeoCoordinates",
            "latitude": enlem,
            "longitude": boylam,
        });
        jsonld["hasMap"] = serde_json::Value::String(format!(
            "https://www.google.com/maps/search/?api=1&query={enlem:.6},{boylam:.6}"
        ));
    }
    if !site.saatler.trim().is_empty() {
        jsonld["openingHours"] = serde_json::Value::String(site.saatler.clone());
    }
    if let Some(yildiz) = site.yildiz {
        let mut ozet = serde_json::json!({
            "@type": "AggregateRating",
            "ratingValue": yildiz,
        });
        if let Some(adet) = site.yorum {
            ozet["reviewCount"] = serde_json::json!(adet);
        }
        jsonld["aggregateRating"] = ozet;
    }''',
    "sunucu JSON-LD geo",
)
# testlerdeki ornek() tam literali yeni alanlari ister
degistir(
    'src/bin/sunucu.rs',
    "            sema: String::new(),",
    '''            sema: String::new(),
            enlem: Some(38.6123),
            boylam: Some(27.4321),
            saatler: "Mo-Su 10:00-23:00".to_string(),
            yildiz: None,
            yorum: None,''',
    "sunucu ornek() alanlari",
)
degistir(
    'src/bin/sunucu.rs',
    '        assert_eq!(sema_tipi("bilinmeyen"), "LocalBusiness");\n    }',
    '''        assert_eq!(sema_tipi("bilinmeyen"), "LocalBusiness");
    }

    #[test]
    fn jsonld_harita_pinini_geo_olarak_gomer() {
        let site = ornek();
        let html = sayfa(&site, "https://ornek.com");
        let bas = html.find("application/ld+json\\">").unwrap() + "application/ld+json\\">".len();
        let son = html[bas..].find("</script>").unwrap() + bas;
        let deger: serde_json::Value =
            serde_json::from_str(&html[bas..son]).expect("geçerli JSON-LD");
        assert_eq!(deger["geo"]["@type"], "GeoCoordinates");
        assert!((deger["geo"]["latitude"].as_f64().unwrap() - 38.6123).abs() < 1e-9);
        assert!((deger["geo"]["longitude"].as_f64().unwrap() - 27.4321).abs() < 1e-9);
        assert!(deger["hasMap"]
            .as_str()
            .unwrap_or("")
            .contains("38.612300,27.432100"));
        assert_eq!(deger["openingHours"], "Mo-Su 10:00-23:00");
    }''',
    "sunucu geo testi",
)

# ---------------------------------------------------------------------------
# 4) main.rs — modül + profil alanları + kanal tipi
# ---------------------------------------------------------------------------
degistir('src/main.rs', "mod onaylanan;", "mod onaylanan;\nmod profil;", "main.rs mod profil")
degistir(
    'src/main.rs',
    "    seo_kanal: Option<std::sync::mpsc::Receiver<Result<crate::isletme_linki::Konum, String>>>,",
    "    seo_kanal: Option<std::sync::mpsc::Receiver<Result<crate::isletme_linki::Isletme, String>>>,",
    "main.rs kanal tipi",
)
degistir(
    'src/main.rs',
    "    /// Link alanındaki son değişikliğin anı (yazma bitene kadar bekleme).\n    seo_son_degisim: f64,",
    '''    /// Link alanındaki son değişikliğin anı (yazma bitene kadar bekleme).
    seo_son_degisim: f64,
    /// Google Haritalar profil kaydı (adres, telefon, saatler, yorum).
    seo_kayit: Option<crate::google::BizKayit>,
    /// [2]/[3] için Local SEO denetimi (kategori, NAP, koordinat, ...).
    seo_denetim: Option<crate::profil::Denetim>,''',
    "main.rs profil alanlari",
)
degistir(
    'src/main.rs',
    "            seo_son_degisim: 0.0,",
    "            seo_son_degisim: 0.0,\n            seo_kayit: None,\n            seo_denetim: None,",
    "main.rs profil init",
)

# ---------------------------------------------------------------------------
# 5) katmanlar.rs — arka plan Isletme, denetim, NAP aktarımı, ekranlar
# ---------------------------------------------------------------------------
p = 'src/katmanlar.rs'
s = oku(p)

# 5a. kanal sonucu artık Isletme
degistir(
    p,
    "                Ok(Ok(konum)) => self.seo_konum_geldi(konum),",
    "                Ok(Ok(isletme)) => self.seo_konum_geldi(isletme),",
    "katmanlar kanal sonucu",
)

# 5b. arka plan işi tam işletme çözsün
s = oku(p)
if "crate::isletme_linki::isletme_coz(&link)" not in s:
    degistir(
        p,
        "let _ = gonderen.send(crate::isletme_linki::konum_coz(&link));",
        "let _ = gonderen.send(crate::isletme_linki::isletme_coz(&link));",
        "katmanlar arka plan isletme_coz",
    )

# 5c. konum geldi: denetim hesapla
degistir(
    p,
    '''    /// Arka plandan konum geldi: harita işareti tazelenir ve SEO motoru
    /// (meta + JSON-LD + sayfa + sitemap) kendiliğinden çalışır.
    fn seo_konum_geldi(&mut self, konum: crate::isletme_linki::Konum) {
        let satir = format!("[✓] Konum çözüldü: {} · {}", konum.ad, konum.koordinat());
        self.logs.push(satir);
        self.seo_konum = Some(konum);
        self.seo_isletmesini_ekle();
    }''',
    '''    /// Arka plandan işletme geldi: harita işareti tazelenir, harita profili
    /// denetlenir ve SEO motoru (meta + JSON-LD + sayfa + sitemap) çalışır.
    fn seo_konum_geldi(&mut self, isletme: crate::isletme_linki::Isletme) {
        let satir = format!(
            "[✓] Konum çözüldü: {} · {}",
            isletme.konum.ad,
            isletme.konum.koordinat()
        );
        self.logs.push(satir);

        let sektor = crate::isletme_linki::sektor_tahmini(&isletme.konum.ad);
        let stok_foto = crate::seo_motoru::stok_medya(&crate::sektor_medya::slug(&sektor)).len();
        let (adres, telefon, saatler, yildiz, yorum) = match isletme.kayit.as_ref() {
            Some(kayit) => (
                kayit.adres.clone(),
                kayit.telefon.clone(),
                kayit.saatler.clone(),
                kayit.yildiz,
                kayit.inceleme,
            ),
            None => (String::new(), String::new(), String::new(), None, None),
        };
        let denetim = crate::profil::denetle(&crate::profil::Girdi {
            ad: &isletme.konum.ad,
            sektor: &sektor,
            enlem: Some(isletme.konum.enlem),
            boylam: Some(isletme.konum.boylam),
            adres: &adres,
            telefon: &telefon,
            saatler: &saatler,
            yildiz,
            yorum,
            stok_foto,
        });
        self.logs.push(format!(
            "[i] Harita profili: {} · {} öneri",
            denetim.ozet(),
            denetim.oneriler().len()
        ));
        self.seo_durum = format!("Profil denetimi: {}", denetim.ozet());
        self.seo_denetim = Some(denetim);
        self.seo_kayit = isletme.kayit.clone();
        self.seo_konum = Some(isletme.konum);
        self.seo_isletmesini_ekle();
    }''',
    "katmanlar seo_konum_geldi denetim",
)

# 5d. NAP/saat/yorum/yıldız profile yazılsın
degistir(
    p,
    '''        let lead = crate::BusinessLead {
            name: konum.ad.clone(),
            address: String::new(),
            category: sektor.clone(),
            phone: String::new(),
            selected: true,
            kaynak,
            konum: Some((konum.enlem, konum.boylam)),
            calisma_saatleri: String::new(),
            demo_yolu: None,
            mesaj_hazir: false,
            kayit_dolulugu: 0,
            site_linki: String::new(),
            son_guncelleme_gun: None,
            inceleme_sayisi: None,
            yildiz_puani: None,''',
    '''        let (adres, telefon, saatler, yildiz, yorum, doluluk) = match self.seo_kayit.as_ref() {
            Some(kayit) => (
                kayit.adres.clone(),
                kayit.telefon.clone(),
                kayit.saatler.clone(),
                kayit.yildiz,
                kayit.inceleme,
                kayit.doluluk,
            ),
            None => (String::new(), String::new(), String::new(), None, None, 0),
        };
        let lead = crate::BusinessLead {
            name: konum.ad.clone(),
            address: adres,
            category: sektor.clone(),
            phone: telefon,
            selected: true,
            kaynak,
            konum: Some((konum.enlem, konum.boylam)),
            calisma_saatleri: saatler,
            demo_yolu: None,
            mesaj_hazir: false,
            kayit_dolulugu: doluluk,
            site_linki: String::new(),
            son_guncelleme_gun: None,
            inceleme_sayisi: yorum,
            yildiz_puani: yildiz,''',
    "katmanlar NAP aktarimi",
)

# 5e. [2] harita altında profil denetimi paneli
degistir(
    p,
    '''                            ui.label(
                                egui::RichText::new("Harita: henüz konum çözülmedi.")
                                    .size(11.0)
                                    .color(DIM),
                            );
                        });
                    }
                }''',
    '''                            ui.label(
                                egui::RichText::new("Harita: henüz konum çözülmedi.")
                                    .size(11.0)
                                    .color(DIM),
                            );
                        });
                    }
                }
                if let Some(denetim) = self.seo_denetim.clone() {
                    ui.add_space(12.0);
                    bolum_etiketi(ui, "HARİTA PROFİL DENETİMİ · LOCAL SEO");
                    ui.label(
                        egui::RichText::new(denetim.ozet())
                            .size(10.5)
                            .color(if denetim.puan >= 80 {
                                YESIL
                            } else if denetim.puan >= 60 {
                                KEHRIBAR
                            } else {
                                KIRMIZI
                            }),
                    );
                    for madde in denetim.maddeler.iter() {
                        let renk = match madde.durum {
                            crate::profil::Durum::Tamam => YESIL,
                            crate::profil::Durum::Uyari => KEHRIBAR,
                            crate::profil::Durum::Eksik => KIRMIZI,
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "· {}: {} · {}",
                                madde.baslik,
                                madde.deger,
                                madde.durum.etiket()
                            ))
                            .size(10.0)
                            .color(renk),
                        );
                    }
                    for oneri in denetim.oneriler() {
                        ui.label(
                            egui::RichText::new(format!("→ {oneri}"))
                                .size(9.5)
                                .color(SOLUK),
                        );
                    }
                }''',
    "katmanlar [2] denetim paneli",
)

# 5f. [3] pin + tamamlanan profil özeti
degistir(
    p,
    '''                if let Some(konum) = self.seo_konum.clone() {
                    let boyut = egui::vec2(ui.available_width(), 130.0);
                    let (rect, _) = ui.allocate_exact_size(boyut, egui::Sense::hover());
                    harita_ciz(ui, rect, &konum, false);
                    ui.add_space(8.0);
                }''',
    '''                if let Some(konum) = self.seo_konum.clone() {
                    let boyut = egui::vec2(ui.available_width(), 130.0);
                    let (rect, _) = ui.allocate_exact_size(boyut, egui::Sense::hover());
                    harita_ciz(ui, rect, &konum, false);
                    ui.add_space(8.0);
                }
                if let Some(denetim) = self.seo_denetim.clone() {
                    bolum_etiketi(ui, "HARİTA PİNİ · PROFİL EKSİKLERİ TAMAMLANDI");
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · pini destekleyen web SEO hazır",
                            denetim.ozet()
                        ))
                        .size(10.0)
                        .color(if denetim.puan >= 80 { YESIL } else { KEHRIBAR }),
                    );
                    for madde in denetim
                        .maddeler
                        .iter()
                        .filter(|madde| madde.durum != crate::profil::Durum::Tamam)
                    {
                        ui.label(
                            egui::RichText::new(format!(
                                "→ {}: {}",
                                madde.baslik, madde.oneri
                            ))
                            .size(9.0)
                            .color(SOLUK),
                        );
                    }
                    ui.add_space(6.0);
                }''',
    "katmanlar [3] profil ozeti",
)
print("TAMAM")
