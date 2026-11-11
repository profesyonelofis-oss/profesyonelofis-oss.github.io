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
# 1) isletme_linki.rs — kısa paylaşım linki + Places ad araması
# ---------------------------------------------------------------------------
YENI_API = '''/// Kısa/paylaşım linki mi? Bunlarda koordinat URL'de olmaz, ağ gerekir.
pub fn kisa_link(link: &str) -> bool {
    let kucuk = link.trim().to_lowercase();
    ["share.google/", "goo.gl/", "maps.app.goo.gl/", "g.co/kgs/"]
        .iter()
        .any(|onek| kucuk.contains(onek))
}

/// Kısa linkin yönlendirmelerini izleyip son adresi döndürür.
fn yonlendirmeyi_izle(link: &str) -> Result<String, String> {
    let ajan = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .redirects(8)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) NexusEnterprise/1.0")
        .build();
    let yanit = ajan
        .get(link)
        .call()
        .map_err(|hata| format!("Paylaşım linki açılamadı: {hata}"))?;
    Ok(yanit.get_url().to_string())
}

/// İşletme adından sektör tahmini (paylaşım linkinde sektör alanı yok).
pub fn sektor_tahmini(ad: &str) -> String {
    let kucuk = ad.to_lowercase();
    let tablo: [(&str, &str); 8] = [
        ("kurtarma", "Çekici"),
        ("çekici", "Çekici"),
        ("cekici", "Çekici"),
        ("kuaför", "Kuaför"),
        ("kuafor", "Kuaför"),
        ("diş", "Diş Kliniği"),
        ("kebap", "Restoran"),
        ("restoran", "Restoran"),
    ];
    for (kelime, sektor) in tablo {
        if kucuk.contains(kelime) {
            return sektor.to_string();
        }
    }
    "Hizmet".to_string()
}

/// Ağlı çözüm: kısa link izlenir; sonuçta koordinat yoksa işletme adı
/// Google Places'ta aranır ve koordinat oradan alınır (tek istek).
pub fn konum_coz(link: &str) -> Result<Konum, String> {
    let link = link.trim();
    if link.is_empty() {
        return Err("Google İşletme linki boş".to_string());
    }
    if !kisa_link(link) {
        return coz(link);
    }
    let son = yonlendirmeyi_izle(link)?;
    if let Ok(konum) = coz(&son) {
        return Ok(konum);
    }
    let ad = ad_bul(&son);
    if ad.is_empty() {
        return Err(format!("Paylaşım linki çözüldü ama işletme adı okunamadı: {son}"));
    }
    let kayit = crate::google::ada_gore(&ad)?;
    let (enlem, boylam) = kayit
        .konum
        .ok_or_else(|| format!("'{ad}' için koordinat alınamadı."))?;
    Ok(Konum {
        ad: kayit.ad,
        enlem,
        boylam,
        place_id: place_id_bul(&kayit.kaynak),
    })
}

'''

YENI_TESTLER = '''
    #[test]
    fn kisa_paylasim_linki_taninir() {
        assert!(kisa_link("https://share.google/RSn6csdDHU9pSBDIn"));
        assert!(kisa_link("https://maps.app.goo.gl/abc123"));
        assert!(!kisa_link(
            "https://www.google.com/maps/place/Kebap+Salonu/@38.61,27.43,17z"
        ));
    }

    #[test]
    fn arama_adresinden_isletme_adi_okunur() {
        let son = "https://www.google.com/search?kgmid=/g/11zxx0wgr5&hl=tr-TR&q=Saruhanl%C4%B1+Oto+Kurtarma&shem=epsd1";
        assert_eq!(ad_bul(son), "Saruhanlı Oto Kurtarma");
    }

    #[test]
    fn sektor_tahmini_oto_kurtarmayi_cekici_sayar() {
        assert_eq!(sektor_tahmini("Saruhanlı Oto Kurtarma"), "Çekici");
        assert_eq!(sektor_tahmini("Kebap Salonu"), "Restoran");
        assert_eq!(sektor_tahmini("Adsız Servis"), "Hizmet");
    }
'''

p = 'src/isletme_linki.rs'
s = oku(p)
if '#[cfg(test)]\nmod testler {' not in s:
    sys.exit("BULUNAMADI: isletme_linki test modulu")
s = s.replace('#[cfg(test)]\nmod testler {', YENI_API + '#[cfg(test)]\nmod testler {', 1)
i = s.rstrip().rfind('\n}')
s = s[:i] + '\n' + YENI_TESTLER + s[i:]
yaz(p, s)
print("degisti: isletme_linki.rs (kisa link + places adi + 3 test)")

# ---------------------------------------------------------------------------
# 2) google.rs — ada göre tek kayıt araması
# ---------------------------------------------------------------------------
AD_ARA = '''    /// Tek işletmeyi ada göre arar (paylaşım linklerinde koordinat yoksa
    /// kullanılır). Bir Places isteği harcar.
    pub fn ad_ara(&self, ad: &str) -> Result<BizKayit, String> {
        match kota_karari(self.gunluk_limit, self.aylik_limit, self.manuel_onay) {
            Karar::Izin => {}
            Karar::GuvenlikMarji => {
                return Err(format!(
                    "Güvenlik marjı: günlük {}/{} istek kullanıldı; ad araması onay bekliyor.",
                    ozet(self.gunluk_limit, self.aylik_limit).gunluk,
                    self.gunluk_limit
                ))
            }
            Karar::GunlukDolu | Karar::AylikDolu => {
                return Err("Google istek sınırı doldu; işletme adı aranamadı.".to_string())
            }
        }
        let _ucus = ucus();
        let govde = serde_json::json!({
            "textQuery": ad.trim(),
            "languageCode": "tr",
            "regionCode": "TR",
            "pageSize": 5,
        });
        let yanit = self
            .rt
            .block_on(places_istek(&self.http, &self.key, &govde))?;
        let (liste, _) = isle(&yanit)?;
        liste
            .into_iter()
            .next()
            .ok_or_else(|| format!("'{ad}' Google kayıtlarında bulunamadı."))
    }

'''

p = 'src/google.rs'
s = oku(p)
kanca = "\n}\n\n/// Kayıtlı anahtarı tek istekle sınar (Geocoding)."
if kanca not in s:
    sys.exit("BULUNAMADI: google.rs impl sonu")
s = s.replace(kanca, "\n\n" + AD_ARA + "}\n\n/// Kayıtlı anahtarı tek istekle sınar (Geocoding).", 1)

ADA_GORE = '''/// Paylaşım linkinden gelen işletme adını Places'ta arar (tek istek).
pub fn ada_gore(ad: &str) -> Result<BizKayit, String> {
    let kasa = crate::anahtar::Kasa::yukle();
    let anahtar = kasa
        .anahtar()
        .ok_or_else(|| {
            "Google API anahtarı yok: üst şeritteki API & Kota Yönetim panelinden girin."
                .to_string()
        })?
        .to_string();
    let ayar = crate::ayar_oku();
    let istemci = Istemci::yeni(
        anahtar,
        ayar.google_gunluk_limit,
        ayar.google_aylik_limit,
        1,
        false,
    )?;
    istemci.ad_ara(ad)
}

'''
kanca = "\n// ---------------------------------------------------------------------------\n// İstekler\n// ---------------------------------------------------------------------------"
if kanca not in s:
    sys.exit("BULUNAMADI: google.rs istekler bolumu")
s = s.replace(kanca, "\n" + ADA_GORE + kanca.lstrip('\n'), 1)
yaz(p, s)
print("degisti: google.rs (ad_ara + ada_gore)")

# ---------------------------------------------------------------------------
# 3) web_veri.rs — medyası olmayan işletme de sayfa alır
# ---------------------------------------------------------------------------
degistir(
    'src/web_veri.rs',
    "/// Kural: demoda seçili (`selected`) ve en az bir medyası olan işletmeler\n/// yayınlanır; medyası boş kayıtlar arşivde kalır, site üretilmez.",
    "/// Kural: seçili (`selected`) her işletme bir sayfa alır. Medyası olmayan\n/// işletmenin galerisi boş kalır; başlık, açıklama, canonical ve JSON-LD yine\n/// üretilir (paylaşım linkinden gelen işletmenin arşivde fotoğrafı olmayabilir).",
    "web_veri.rs json_uret belgesi",
)
degistir(
    'src/web_veri.rs',
    "        if medya.is_empty() {\n            continue;\n        }\n",
    "",
    "web_veri.rs medyasiz eleme",
)

# ---------------------------------------------------------------------------
# 4) main.rs — arka plan link çözümü alanları
# ---------------------------------------------------------------------------
degistir(
    'src/main.rs',
    "    /// [3] önizlemede onaylanıp [4] Gelen'e aktarılan sayfa slug'ları.\n    seo_onaylanan: Vec<String>,",
    "    /// [3] önizlemede onaylanıp [4] Gelen'e aktarılan sayfa slug'ları.\n    seo_onaylanan: Vec<String>,\n    /// Paylaşım linki arka planda çözülürken açık kalan kanal.\n    seo_kanal: Option<std::sync::mpsc::Receiver<Result<crate::isletme_linki::Konum, String>>>,\n    /// [2] sekmesinin durum satırı: çözülüyor / çözüldü / hata.\n    seo_durum: String,",
    "main.rs seo_kanal alanlari",
)
degistir(
    'src/main.rs',
    "            seo_onaylanan: Vec::new(),",
    "            seo_onaylanan: Vec::new(),\n            seo_kanal: None,\n            seo_durum: String::new(),",
    "main.rs seo_kanal init",
)

# ---------------------------------------------------------------------------
# 5) katmanlar.rs — arka plan çözümü, durum satırı, işletmeyi akışa ekleme
# ---------------------------------------------------------------------------
p = 'src/katmanlar.rs'
s = oku(p)

kanca = "    fn seo_alani_ekrani(&mut self, ctx: &egui::Context) {\n"
if kanca not in s:
    sys.exit("BULUNAMADI: seo_alani_ekrani")
POLL = '''    fn seo_alani_ekrani(&mut self, ctx: &egui::Context) {
        // Paylaşım linki arka planda çözülür: arayüz beklemez.
        if let Some(mut kanal) = self.seo_kanal.take() {
            match kanal.try_recv() {
                Ok(Ok(konum)) => {
                    let satir = format!("[✓] Konum çözüldü: {} · {}", konum.ad, konum.koordinat());
                    self.logs.push(satir.clone());
                    self.seo_durum = satir;
                    self.seo_konum = Some(konum);
                }
                Ok(Err(hata)) => {
                    self.logs.push(format!("[!] {hata}"));
                    self.seo_durum = format!("[!] {hata}");
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => self.seo_kanal = Some(kanal),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.seo_durum = "[!] Link çözümü yarıda kaldı.".to_string();
                }
            }
        }
        // Çözüm sürerken her karede yeniden çizim iste.
        if self.seo_kanal.is_some() {
            ctx.request_repaint();
        }
'''
s = s.replace(kanca, POLL, 1)

# KONUMU ÇÖZ düğmesi: ağlı çözüm arka planda
bas = s.find('                    if birincil_buton(ui, "KONUMU ÇÖZ"')
son = s.find('                    if ikincil_buton(ui, "SEO MOTORU"')
if bas == -1 or son == -1 or son < bas:
    sys.exit("BULUNAMADI: KONUMU COZ blogu")
BLOK = '''                    if birincil_buton(ui, "KONUMU ÇÖZ", true, Some(egui::vec2(112.0, 30.0)))
                        .clicked()
                    {
                        let link = self.seo_link.trim().to_string();
                        if crate::isletme_linki::kisa_link(&link) {
                            // Kısa link: yönlendirme + Places araması ağ ister.
                            let (gonderen, alan) = std::sync::mpsc::channel();
                            let istek = link.clone();
                            std::thread::spawn(move || {
                                let _ = gonderen.send(crate::isletme_linki::konum_coz(&istek));
                            });
                            self.seo_kanal = Some(alan);
                            self.seo_durum =
                                format!("Paylaşım linki arka planda çözülüyor… {link}");
                        } else {
                            match crate::isletme_linki::coz(&link) {
                                Ok(konum) => {
                                    let satir = format!(
                                        "[✓] Konum çözüldü: {} · {}",
                                        konum.ad,
                                        konum.koordinat()
                                    );
                                    self.logs.push(satir.clone());
                                    self.seo_durum = satir;
                                    self.seo_konum = Some(konum);
                                }
                                Err(hata) => {
                                    self.logs.push(format!("[!] {hata}"));
                                    self.seo_durum = format!("[!] {hata}");
                                    self.seo_konum = None;
                                }
                            }
                        }
                    }
'''
s = s[:bas] + BLOK + s[son:]

# SEO MOTORU: önce çözülen işletmeyi akışa kat
s = s.replace(
    '                        self.seo_calistir();\n                        self.katman = Katman::SeoOnizleme;',
    '                        self.seo_isletmesini_ekle();\n                        self.katman = Katman::SeoOnizleme;',
    1,
)

# Durum satırı: açıklamanın altında
kanca = "                ui.add_space(8.0);\n                ui.horizontal(|ui| {\n                    let genislik"
if kanca not in s:
    sys.exit("BULUNAMADI: seo_alani durum kancasi")
s = s.replace(
    kanca,
    "                if !self.seo_durum.is_empty() {\n                    ui.label(\n                        egui::RichText::new(&self.seo_durum)\n                            .size(9.5)\n                            .color(if self.seo_durum.starts_with(\"[!\") {\n                                KIRMIZI\n                            } else {\n                                YESIL\n                            }),\n                    );\n                }\n" + kanca,
    1,
)

EK_IMPL = '''
impl LeadApp {
    /// [2]'de çözülen işletmeyi keşif listesine katar ve SEO üretimini
    /// çalıştırır: sayfa `web/veri.json`a yazılır, [3] onu önizler.
    fn seo_isletmesini_ekle(&mut self) {
        let Some(konum) = self.seo_konum.clone() else {
            self.seo_calistir();
            return;
        };
        let sektor = crate::isletme_linki::sektor_tahmini(&konum.ad);
        let zaten_var = self
            .leads
            .iter()
            .any(|lead| lead.name.eq_ignore_ascii_case(&konum.ad));
        if !zaten_var {
            let kaynak = if konum.place_id.is_empty() {
                "https://www.google.com/maps".to_string()
            } else {
                format!(
                    "https://www.google.com/maps/place/?q=place_id:{}",
                    konum.place_id
                )
            };
            self.leads.push(crate::BusinessLead {
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
                yildiz_puani: None,
                durum: crate::musteriler::Durum::Yeni,
                yayin_linki: None,
                karne_linki: None,
                mesaj_varyanti: None,
                foto: None,
                fotolar: Vec::new(),
                medya: Vec::new(),
            });
            self.logs.push(format!(
                "[+] {} SEO akışına eklendi · sektör: {sektor}",
                konum.ad
            ));
        }
        self.seo_calistir();
    }
}
'''
s = s.rstrip() + "\n" + EK_IMPL
yaz(p, s)
print("degisti: katmanlar.rs (arka plan cozum + isletme ekleme)")
print("TAMAM")
