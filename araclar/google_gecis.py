"""OSM/Overpass hattını Google Maps Platform'a çeviren tek seferlik geçiş.

Yapılanlar:
- main.rs: modül listesi, tarama hattı (Places + Geocoding), SEO puanı, kart
  rozeti, API paneli durumu ve çağrıları, ayar alanları.
- katmanlar.rs: canlı kota şeridi + "API & Kota Yönetim" penceresi.
- saglik.rs: açılış denetimi Google uç noktalarına bakar.
- demo_site.rs: gömme OSM haritası yerine yerel kart + Google Maps bağlantısı.
- Cargo.toml: Tokio/reqwest/aes-gcm zaten eklendi; tokio özellikleri genişletilir.
- src/overpass.rs silinir.
"""

import re
from pathlib import Path

kok = Path(".")
rapor = []


def yaz(yol, metin):
    Path(yol).write_text(metin, encoding="utf-8", newline="")


def oku(yol):
    return Path(yol).read_text(encoding="utf-8")


# ---------------------------------------------------------------------------
# main.rs
# ---------------------------------------------------------------------------
m = oku("src/main.rs")

if "mod overpass;" in m:
    m = m.replace("mod overpass;", "mod anahtar;\nmod google;")
    rapor.append("modül listesi güncellendi")

# Üst bilgi notu.
bas = "// İl/ilçe merkezi Nominatim'den"
if bas in m:
    i = m.index(bas)
    j = m.index("// ---------------------------------------------------------------------------", i)
    m = (
        m[:i]
        + """// İl/ilçe merkezi Google Geocoding'den, işletmeler Google Places API (New)
// Text Search'ten okunur. Anahtar şifreli kasada tutulur; her istek kotadan
// pay ayırır, aynı ilçe tekrar taranırken sonuç önbellekten gelir.
"""
        + m[j:]
    )
    rapor.append("motor üst notu güncellendi")

# LeadApp alanları.
m = m.replace(
    "    baslangic: saglik::Denetim,\n",
    """    baslangic: saglik::Denetim,
    /// API & Kota Yönetim penceresi: anahtar girişi, test ve canlı sayaç.
    api_panel: bool,
    api_girdi: String,
    api_mesaj: String,
    api_test: Option<Receiver<Result<String, String>>>,
    /// Panelde düzenlenen istek sınırları (ayarlar.json'a yazılır).
    api_gunluk: usize,
    api_aylik: usize,
    api_sayfa: usize,
""",
    1,
)

m = m.replace(
    "            harita_imza: (0, None),\n",
    """            harita_imza: (0, None),
            api_panel: false,
            api_girdi: String::new(),
            api_mesaj: String::new(),
            api_test: None,
            api_gunluk: ayar.google_gunluk_limit,
            api_aylik: ayar.google_aylik_limit,
            api_sayfa: ayar.google_sayfa,
""",
    1,
)

# Ayar yapısına istek bütçesi alanları.
m = m.replace(
    "    demo_fotolar: Vec<String>,\n}",
    """    demo_fotolar: Vec<String>,
    /// Google API istek bütçesi: günlük sınır ve aylık sınır.
    #[serde(default = "google_gunluk_varsayilan")]
    google_gunluk_limit: usize,
    #[serde(default = "google_aylik_varsayilan")]
    google_aylik_limit: usize,
    /// Tarama başına okunacak sayfa sayısı (1-3 · sayfa başına 20 kayıt).
    #[serde(default = "google_sayfa_varsayilan")]
    google_sayfa: usize,
}""",
    1,
)

m = m.replace(
    "            demo_fotolar: Vec::new(),\n        }",
    """            demo_fotolar: Vec::new(),
            google_gunluk_limit: google_gunluk_varsayilan(),
            google_aylik_limit: google_aylik_varsayilan(),
            google_sayfa: google_sayfa_varsayilan(),
        }""",
    1,
)

m = m.replace(
    "fn evet() -> bool {\n    true\n}",
    """fn evet() -> bool {
    true
}

/// Google istek bütçesi varsayılanları: günlük 20 istek, aylık 400 istek.
/// Aylık 200 $ ücretsiz kredinin çok altında kalır (10.000 istek ≈ tüm kredi).
fn google_gunluk_varsayilan() -> usize {
    crate::google::VARSAYILAN_GUNLUK
}

fn google_aylik_varsayilan() -> usize {
    crate::google::VARSAYILAN_AYLIK
}

fn google_sayfa_varsayilan() -> usize {
    crate::google::VARSAYILAN_SAYFA
}""",
    1,
)

# ayar_olustur içine yeni alanlar.
if "demo_fotolar: self.tema_fotolar.clone()," in m:
    m = m.replace(
        "demo_fotolar: self.tema_fotolar.clone(),",
        """demo_fotolar: self.tema_fotolar.clone(),
            google_gunluk_limit: self.api_gunluk,
            google_aylik_limit: self.api_aylik,
            google_sayfa: self.api_sayfa,""",
        1,
    )
    rapor.append("ayar_olustur güncellendi")
else:
    rapor.append("UYARI: ayar_olustur alanı bulunamadı")

# SEO puanı yardımcısı.
m = m.replace(
    "fn potansiyel_puani(isletme: &BusinessLead) -> u8 {",
    """/// 100 üzerinden SEO puanı: yalnızca Google kaydındaki gerçek alanlardan
/// hesaplanır (telefon, adres, saatler, yıldız, inceleme). Web sitesi yokluğu
/// bu ekranın hedefidir ve puana +25 olarak yansır.
fn seo_puani(isletme: &BusinessLead) -> u8 {
    google::seo_puani(
        &isletme.address,
        &isletme.phone,
        &isletme.calisma_saatleri,
        isletme.yildiz_puani,
        isletme.inceleme_sayisi,
    )
}

fn potansiyel_puani(isletme: &BusinessLead) -> u8 {""",
    1,
)

# Kart rozetine SEO pili.
m = m.replace(
    '                            pill(ui, "❌ Web Sitesi Yok - Hedef Müşteri", VURGU);\n',
    """                            pill(ui, "❌ Web Sitesi Yok - Hedef Müşteri", VURGU);
                            let seo = seo_puani(lead);
                            pill(
                                ui,
                                &format!("SEO {seo}/100"),
                                if seo >= 70 { YESIL } else { MAVI },
                            );
""",
    1,
)

# Kayıt bağlantısı artık Google kaydı.
m = m.replace('"OSM kaydını aç ↗"', '"Google kaydını aç ↗"')

# Harita alanı açıklaması.
m = m.replace(
    "/// Katman 1'in interaktif haritası (OpenStreetMap döşemeleri).",
    "/// Katman 1'in interaktif haritası (yerel koordinat ızgarası, ağ isteği yok).",
)

# Anahtar testi ve panel çağrıları.
m = m.replace(
    "        while let Ok(mesaj) = self.log_rx.try_recv() {",
    "        self.api_test_poll();\n        while let Ok(mesaj) = self.log_rx.try_recv() {",
    1,
)
m = m.replace(
    "        self.kota_seridi(ctx);\n",
    "        self.kota_seridi(ctx);\n        self.api_paneli(ctx);\n",
    1,
)

# --- Tarama hattı: Overpass yerine Google -------------------------------
bas = "fn tarama_calistir("
son = "#[cfg(test)]\nmod testler {"
if bas in m and son in m:
    i = m.index(bas)
    j = m.index(son, i)
    m = (
        m[:i]
        + '''/// Google Places + Geocoding taraması. Anahtar şifreli kasadan okunur;
/// yoksa tarama başlamaz ve kullanıcı API & Kota Yönetim paneline yönlendirilir.
fn tarama_calistir(tx: Sender<Mesaj>, il: String, ilce: Option<String>, sektor: String) {
    let ayar = ayar_oku();
    let kasa = anahtar::Kasa::yukle();
    let Some(anahtar) = kasa.anahtar().map(str::to_string) else {
        let _ = tx.send(Mesaj::Log(
            "[!] Google API anahtarı yok: üst şeritteki 'API & KOTA YÖNETİM' düğmesinden anahtarınızı kaydedin."
                .to_string(),
        ));
        let _ = tx.send(Mesaj::TaramaBasarisiz);
        return;
    };
    let istemci = match google::Istemci::yeni(
        anahtar,
        ayar.google_gunluk_limit,
        ayar.google_aylik_limit,
        ayar.google_sayfa,
    ) {
        Ok(istemci) => istemci,
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] {hata}")));
            let _ = tx.send(Mesaj::TaramaBasarisiz);
            return;
        }
    };
    let ozet = google::ozet(istemci.gunluk_limit, istemci.aylik_limit);
    let _ = tx.send(Mesaj::Log(format!(
        "[i] Google API hazır · günlük {}/{} istek · sayfa {}/tarama.",
        ozet.gunluk, ozet.gunluk_limit, istemci.sayfa_siniri
    )));

    let mut konum_onbellek = KonumOnbellek::yukle();
    let ilce_adi = ilce.clone().unwrap_or_default();
    let konum = onbellek_konum(&istemci, &mut konum_onbellek, &il, &ilce_adi, &tx);
    if let Some(konum) = &konum {
        // Kamera seçilen bölgeye uçar: ilçe sokak ölçeği, il geneli ülke ölçeği.
        let _ = tx.send(Mesaj::Odakla {
            enlem: konum.enlem,
            boylam: konum.boylam,
            zoom: if ilce.is_some() { 13 } else { 9 },
        });
    }
    match ilce {
        Some(ilce) => tarama_tek_ilce(&tx, &istemci, &il, &ilce, &sektor, konum.as_ref()),
        None => tarama_il_geneli(&tx, &istemci, &il, &sektor, konum.as_ref()),
    }
}

/// İl/ilçe merkezini Google Geocoding'den çözer; sonuç `konum_onbellek.json`a
/// yazılır, aynı bölge bir daha faturalandırılmaz (180 gün).
fn onbellek_konum(
    istemci: &google::Istemci,
    onbellek: &mut KonumOnbellek,
    il: &str,
    ilce: &str,
    tx: &Sender<Mesaj>,
) -> Option<Konum> {
    if !ilce.trim().is_empty() {
        if let Some(konum) = onbellek.al(il, ilce) {
            return Some(konum);
        }
    }
    match istemci.geocode(il, ilce) {
        Ok((enlem, boylam, yaricap_m)) => {
            let konum = Konum {
                enlem,
                boylam,
                yaricap_m,
            };
            let kayit_adi = if ilce.trim().is_empty() {
                "il merkezi"
            } else {
                ilce
            };
            onbellek.ekle(il, kayit_adi, &konum);
            onbellek.kaydet();
            let _ = tx.send(Mesaj::Log(format!(
                "[i] Google Geocoding: {enlem:.4}, {boylam:.4} · ~{} km yarıçap.",
                (yaricap_m / 1000).max(1)
            )));
            Some(konum)
        }
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] Google Geocoding: {hata}")));
            None
        }
    }
}

/// Tek ilçe taraması: önbellek → Google Places.
fn tarama_tek_ilce(
    tx: &Sender<Mesaj>,
    istemci: &google::Istemci,
    il: &str,
    ilce: &str,
    sektor: &str,
    konum: Option<&Konum>,
) {
    let mut onbellek = SonucOnbellek::yukle();
    let merkez = konum
        .map(|konum| {
            format!(
                " · merkez {:.4}, {:.4} · ~{} km",
                konum.enlem,
                konum.boylam,
                (konum.yaricap_m / 1000).max(1)
            )
        })
        .unwrap_or_default();
    let _ = tx.send(Mesaj::Log(format!(
        "[>] Google Places: {il} / {ilce} · {sektor} (en fazla {SONUC_SINIRI} kayıt){merkez}..."
    )));
    match ilce_ara(tx, istemci, &mut onbellek, il, ilce, sektor) {
        Ok((liste, onbellekten)) => {
            if !onbellekten && !liste.is_empty() {
                let _ = tx.send(Mesaj::Log(
                    "[i] Veri kaynağı: Google Places API (New) · Geocoding · yalnızca sitesiz işletmeler."
                        .to_string(),
                ));
            }
            let yuksek = liste
                .iter()
                .filter(|isletme| potansiyel_puani(isletme) >= SICAK_ESIK)
                .count();
            let _ = tx.send(Mesaj::Log(format!(
                "[✓] {} sitesiz potansiyel müşteri bulundu · {yuksek} sıcak fırsat.",
                liste.len()
            )));
            let _ = tx.send(Mesaj::TaramaBitti(liste));
        }
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] {ilce} taranamadı: {hata}")));
            let _ = tx.send(Mesaj::TaramaBasarisiz);
        }
    }
}

/// İlçe sonucu: önce yerel önbellek, yoksa Google Places. Dönen bayrak
/// sonucun önbellekten geldiğini söyler (günlükte ayrı yazılır).
fn ilce_ara(
    tx: &Sender<Mesaj>,
    istemci: &google::Istemci,
    onbellek: &mut SonucOnbellek,
    il: &str,
    ilce: &str,
    sektor: &str,
) -> Result<(Vec<BusinessLead>, bool), String> {
    if let Some(kayit) = onbellek.al(il, ilce, sektor) {
        let _ = tx.send(Mesaj::Log(format!(
            "[i] {ilce}: sonuçlar önbellekten alındı ({} kayıt, {} gün önce); Google'a istek atılmadı.",
            kayit.leads.len(),
            kayit.gun
        )));
        return Ok((kayit.leads, true));
    }
    let sonuc = istemci.ara(il, ilce, sektor)?;
    if let Some(neden) = &sonuc.durduruldu {
        let _ = tx.send(Mesaj::Log(format!("[!] {neden}")));
    }
    let _ = tx.send(Mesaj::Log(format!(
        "[i] Google Places: {} ham kayıt · {} web siteli işletme elendi · {} sayfa.",
        sonuc.kayitlar.len() + sonuc.siteli,
        sonuc.siteli,
        sonuc.sayfa
    )));
    let mut leads: Vec<BusinessLead> = sonuc
        .kayitlar
        .into_iter()
        .map(|kayit| isletmeye_cevir(kayit, sektor))
        .collect();
    // Telefonu olmayan kayıt müşteri adayı değildir.
    leads.retain(|lead| telefon_gecerli(&lead.phone));
    incelemeye_gore_sirala(&mut leads);
    leads_sirala(&mut leads);
    leads.truncate(SONUC_SINIRI);
    // Kota engeli yüzünden boş dönen sonuç önbelleğe yazılmaz: yarın aynı
    // bölge yeniden denenebilmeli.
    if !(leads.is_empty() && sonuc.durduruldu.is_some()) {
        onbellek.ekle(il, ilce, sektor, &leads, sonuc.siteli);
        onbellek.kaydet();
    }
    Ok((leads, false))
}

/// İl geneli: tek Places sorgusu ile ilin tamamı aranır; ilçe ilçe gezmek
/// düzinelerce faturalı istek üretirdi.
fn tarama_il_geneli(
    tx: &Sender<Mesaj>,
    istemci: &google::Istemci,
    il: &str,
    sektor: &str,
    _konum: Option<&Konum>,
) {
    let _ = tx.send(Mesaj::Ilerleme {
        tamamlanan: 0,
        toplam: 1,
    });
    let _ = tx.send(Mesaj::Log(format!(
        "[>] {il} il geneli: Google Places tek sorguda ili tarıyor ({sektor})..."
    )));
    let mut onbellek = SonucOnbellek::yukle();
    let bulunanlar = match ilce_ara(tx, istemci, &mut onbellek, il, "", sektor) {
        Ok((liste, _)) => liste,
        Err(hata) => {
            let _ = tx.send(Mesaj::Log(format!("[!] {il} taranamadı: {hata}")));
            let _ = tx.send(Mesaj::TaramaBasarisiz);
            return;
        }
    };
    let _ = tx.send(Mesaj::Ilerleme {
        tamamlanan: 1,
        toplam: 1,
    });
    let yuksek = bulunanlar
        .iter()
        .filter(|isletme| potansiyel_puani(isletme) >= SICAK_ESIK)
        .count();
    let _ = tx.send(Mesaj::Log(format!(
        "[✓] {il} il geneli bitti: {} sitesiz kayıt ({} sıcak fırsat); telefonu olmayan kayıt listeye alınmadı.",
        bulunanlar.len(),
        yuksek
    )));
    let _ = tx.send(Mesaj::TaramaBitti(bulunanlar));
}

/// Google kaydını arayüzün satır modeline çevirir.
fn isletmeye_cevir(kayit: google::BizKayit, sektor: &str) -> BusinessLead {
    BusinessLead {
        name: kayit.ad,
        address: kayit.adres,
        category: sektor.to_string(),
        phone: kayit.telefon,
        selected: true,
        kaynak: kayit.kaynak,
        konum: kayit.konum,
        calisma_saatleri: kayit.saatler,
        demo_yolu: None,
        mesaj_hazir: false,
        kayit_dolulugu: kayit.doluluk,
        site_linki: String::new(),
        son_guncelleme_gun: None,
        inceleme_sayisi: kayit.inceleme,
        yildiz_puani: kayit.yildiz,
        durum: musteriler::Durum::Yeni,
        yayin_linki: None,
        karne_linki: None,
        mesaj_varyanti: None,
        foto: None,
        fotolar: Vec::new(),
    }
}

'''
        + m[j:]
    )
    rapor.append("tarama hattı Google'a çevrildi")
else:
    rapor.append("UYARI: tarama hattı dilimi bulunamadı")

yaz("src/main.rs", m)

# ---------------------------------------------------------------------------
# katmanlar.rs
# ---------------------------------------------------------------------------
k = oku("src/katmanlar.rs")
bas = "    /// Katman 1'in üst şeridi"
son = "    /// Katman 1'in ortasındaki interaktif harita"
if bas in k and son in k:
    i = k.index(bas)
    j = k.index(son, i)
    yeni_serit = '''    /// Katman 1'in üst şeridi: canlı Google API kota göstergesi, anahtar
    /// durumu ve WhatsApp günlük freni. Saniyede iki kez tazelenir.
    pub(crate) fn kota_seridi(&mut self, ctx: &egui::Context) {
        let ozet = google::ozet(self.api_gunluk, self.api_aylik);
        let kasa = anahtar::Kasa::yukle();
        let bugun = bugun_anahtari();
        let gonderilen = self.defter.gun_gonderim(&bugun);
        egui::TopBottomPanel::top("kota_seridi")
            .exact_height(50.0)
            .frame(
                egui::Frame::none()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(22.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    pill(ui, "GOOGLE MAPS API", VURGU);
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
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let kalan = GUNLUK_GONDERIM_LIMITI.saturating_sub(gonderilen);
                        let renk = if kalan == 0 {
                            KIRMIZI
                        } else if kalan <= 3 {
                            TURUNCU
                        } else {
                            YESIL
                        };
                        pill(
                            ui,
                            &format!("WhatsApp {gonderilen}/{GUNLUK_GONDERIM_LIMITI}"),
                            renk,
                        );
                        ui.add_space(8.0);
                        if ui.add(api_dugmesi("API & KOTA YÖNETİM")).clicked() {
                            self.api_panel = true;
                            self.api_mesaj.clear();
                        }
                    });
                });
            });
        if self.is_running || self.api_panel {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
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
                                self.api_mesaj =
                                    "✓ Anahtar şifrelenerek kaydedildi.".to_string();
                                self.api_girdi.clear();
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
                            Ok(()) => self.api_mesaj = "Anahtar silindi.".to_string(),
                            Err(hata) => self.api_mesaj = format!("[!] {hata}"),
                        }
                    }
                });
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.add(
                    egui::ProgressBar::new(ozet.gun_yuzde as f32)
                        .text(format!(
                            "Günlük {} / {}  ·  kalan {}",
                            ozet.gunluk, ozet.gunluk_limit, ozet.kalan_gun
                        ))
                        .desired_width(508.0),
                );
                ui.add(
                    egui::ProgressBar::new(ozet.ay_yuzde as f32)
                        .text(format!(
                            "Aylık {} / {}  ·  kalan {}",
                            ozet.aylik, ozet.aylik_limit, ozet.kalan_ay
                        ))
                        .desired_width(508.0),
                );
                ui.add(
                    egui::ProgressBar::new(ozet.ucretsiz_yuzde as f32)
                        .text(format!(
                            "Aylık ücretsiz kota kullanımı %{:.2} ({} / {} istek)",
                            ozet.ucretsiz_yuzde * 100.0,
                            ozet.aylik,
                            google::AYLIK_UCRETSIZ_ISTEK
                        ))
                        .desired_width(508.0),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Oturumda atılan istek: {} · kurulumdan bu yana: {}",
                        ozet.oturum, ozet.toplam
                    ))
                    .size(9.5)
                    .color(DIM),
                );
                if ozet.seviye != google::Seviye::Normal {
                    ui.label(
                        egui::RichText::new(
                            "⚠ Maliyet uyarısı: istek sınırına yaklaşıldı; yeni taramalar duracak.",
                        )
                        .size(9.5)
                        .color(KIRMIZI),
                    );
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Günlük").size(9.5).color(DIM));
                    ui.add(egui::DragValue::new(&mut self.api_gunluk).range(1..=500));
                    ui.label(egui::RichText::new("Aylık").size(9.5).color(DIM));
                    ui.add(egui::DragValue::new(&mut self.api_aylik).range(10..=10_000));
                    ui.label(egui::RichText::new("Sayfa").size(9.5).color(DIM));
                    ui.add(egui::DragValue::new(&mut self.api_sayfa).range(1..=3));
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

'''
    k = k[:i] + yeni_serit + k[j:]
    k += '''
/// Kota şeridindeki vurgulu düğme (API & Kota Yönetim).
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
'''
    k = k.replace(
        '"{} · {} kayıt · {} koordinatsız · OpenStreetMap"',
        '"{} · {} kayıt · {} koordinatsız · Google Places"',
    )
    yaz("src/katmanlar.rs", k)
    rapor.append("kota şeridi ve API paneli eklendi")
else:
    rapor.append("UYARI: kota_seridi dilimi bulunamadı")

# ---------------------------------------------------------------------------
# saglik.rs
# ---------------------------------------------------------------------------
g = oku("src/saglik.rs")
g = g.replace('"Harita motoru ve API",', '"Google API ve harita",')
g = g.replace(
    'pub const HARITA_HATA: &str =\n    "Başarısız! İnternet bağlantısı yok veya harita servisine erişilemiyor.";',
    'pub const HARITA_HATA: &str =\n    "Başarısız! İnternet bağlantısı yok veya Google API uç noktalarına erişilemiyor.";',
)
bas = "/// OpenStreetMap erişimi"
son = "/// Dosya yoksa/boşsa sorun değil"
if bas in g and son in g:
    i = g.index(bas)
    j = g.index(son, i)
    g = (
        g[:i]
        + '''/// Google Maps Platform erişimi: Places ve Geocoding uç noktaları.
///
/// Anahtar burada kullanılmaz; yalnızca ağ erişimi sınanır. Anahtarsız
/// kurulumda da denetim geçer, tarama başlarken panel yönlendirir.
fn harita() -> Result<String, String> {
    let erisim = |adres: &str| -> bool {
        let Ok(mut adresler) = adres.to_socket_addrs() else {
            return false;
        };
        let Some(soket) = adresler.next() else {
            return false;
        };
        TcpStream::connect_timeout(&soket, Duration::from_secs(3)).is_ok()
    };
    if !(erisim("places.googleapis.com:443") || erisim("maps.googleapis.com:443")) {
        return Err(HARITA_HATA.to_string());
    }
    Ok("Google API hazır: Places ve Geocoding uç noktalarına erişim var.".to_string())
}

'''
        + g[j:]
    )
    yaz("src/saglik.rs", g)
    rapor.append("sağlık denetimi Google'a bağlandı")
else:
    rapor.append("UYARI: saglik harita dilimi bulunamadı")

# ---------------------------------------------------------------------------
# demo_site.rs
# ---------------------------------------------------------------------------
d = oku("src/demo_site.rs")
bas = "fn harita_html(g: &Girdi, ad: &str) -> String {"
son = "fn saatler_html(g: &Girdi) -> String {"
if bas in d and son in d:
    i = d.index(bas)
    j = d.index(son, i)
    d = (
        d[:i]
        + '''fn harita_html(g: &Girdi, ad: &str) -> String {
    let Some((enlem, boylam)) = g.konum else {
        return String::new();
    };
    // Demo sayfası ağ isteği yapmaz: harita yerine koordinat kartı ve
    // Google Maps bağlantısı verilir (gömme harita kotadan istek harcamaz).
    let ac = format!("https://www.google.com/maps?q={enlem:.5},{boylam:.5}");
    format!(
        r##"<div class="harita">
<div class="harita-yerel" style="display:flex;flex-direction:column;gap:6px;align-items:center;justify-content:center;min-height:230px;border:1px solid currentColor;border-radius:16px;padding:22px;text-align:center">
<span style="font-size:11px;letter-spacing:.18em;text-transform:uppercase;opacity:.62">Konum</span>
<b style="font-size:17px">{ad}</b>
<small style="font-size:12px;opacity:.72">{enlem:.5}, {boylam:.5}</small>
</div>
<div class="harita-alt"><span>Manisa · Türkiye</span><a href="{ac}" target="_blank" rel="noopener">Google Maps'te aç ↗</a></div>
</div>"##
    )
}

'''
        + d[j:]
    )
    yaz("src/demo_site.rs", d)
    rapor.append("demo haritası yerel karta çevrildi")
else:
    rapor.append("UYARI: demo harita dilimi bulunamadı")

# ---------------------------------------------------------------------------
# iller.rs yorumu
# ---------------------------------------------------------------------------
y = oku("src/iller.rs")
if "Nominatim" in y:
    y = y.replace("Nominatim", "Google Geocoding")
    yaz("src/iller.rs", y)
    rapor.append("iller.rs yorumu güncellendi")

# ---------------------------------------------------------------------------
# Cargo.toml: tokio özellikleri
# ---------------------------------------------------------------------------
c = oku("Cargo.toml")
c = c.replace(
    'tokio = { version = "1", features = ["rt-multi-thread"] }',
    'tokio = { version = "1", features = ["rt-multi-thread", "time", "net"] }',
)
yaz("Cargo.toml", c)

# ---------------------------------------------------------------------------
# .gitignore
# ---------------------------------------------------------------------------
gi = oku(".gitignore") if Path(".gitignore").exists() else ""
for satir in ["api_anahtari.enc", "api_tuz.bin", "google_kota.json"]:
    if satir not in gi:
        gi = gi.rstrip("\n") + "\n" + satir + "\n"
yaz(".gitignore", gi)

# ---------------------------------------------------------------------------
# overpass.rs sil
# ---------------------------------------------------------------------------
if Path("src/overpass.rs").exists():
    Path("src/overpass.rs").unlink()
    rapor.append("src/overpass.rs silindi")

print("\n".join(rapor))

kalan = [
    (yol, no, satir.strip())
    for yol in ["src/main.rs", "src/katmanlar.rs", "src/saglik.rs", "src/demo_site.rs", "src/iller.rs", "src/musteriler.rs"]
    for no, satir in enumerate(oku(yol).split("\n"), 1)
    if "overpass" in satir.lower() or "OpenStreetMap" in satir or "OSM" in satir
]
print(f"kalan OSM/Overpass satırı: {len(kalan)}")
for yol, no, satir in kalan[:20]:
    print(f"  {yol}:{no}: {satir[:110]}")
