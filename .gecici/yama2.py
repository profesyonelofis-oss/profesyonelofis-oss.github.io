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
# 1) web_veri.rs — yayındaki siteleri silmeden tek sayfa ekle/güncelle
# ---------------------------------------------------------------------------
TEK_SITE = '''/// Tek işletmenin SEO sayfasını yayındaki listeye ekler (aynı slug varsa
/// günceller) ve slug'ı döndürür. Diğer yayın sayfalarına dokunmaz: [2]
/// sekmesinden gelen işletme, keşif taramasının ürettiği siteleri silmez.
pub fn tek_site_ekle(lead: &BusinessLead, il: &str, ilce: &str) -> Result<String, String> {
    let json = json_uret(std::slice::from_ref(lead), il, ilce);
    let deger: serde_json::Value =
        serde_json::from_str(&json).map_err(|hata| format!("json: {hata}"))?;
    let site = deger
        .get("siteler")
        .and_then(|s| s.as_array())
        .and_then(|s| s.first())
        .cloned()
        .ok_or_else(|| "SEO sayfası üretilemedi.".to_string())?;
    let slug = site
        .get("slug")
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .to_string();
    if slug.is_empty() {
        return Err("SEO sayfası için slug üretilemedi.".to_string());
    }
    let _kilit = YAZMA_KILIDI.lock().map_err(|_| "kilit".to_string())?;
    let mevcut = std::fs::read_to_string(VERI_YOLU).unwrap_or_default();
    let mut kok: serde_json::Value = serde_json::from_str(&mevcut)
        .unwrap_or_else(|_| serde_json::json!({"surum": 1, "siteler": []}));
    kok["guncelleme"] =
        serde_json::json!(chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string());
    {
        let dizi = kok
            .get_mut("siteler")
            .and_then(|s| s.as_array_mut())
            .ok_or_else(|| "web/veri.json biçimi bozuk.".to_string())?;
        dizi.retain(|kayit| kayit.get("slug").and_then(|s| s.as_str()) != Some(slug.as_str()));
        dizi.insert(0, site);
    }
    if let Some(klasor) = Path::new(VERI_YOLU).parent() {
        std::fs::create_dir_all(klasor).map_err(|hata| format!("web klasörü: {hata}"))?;
    }
    std::fs::create_dir_all(".gecici").ok();
    let gecici = ".gecici/web_veri.tmp";
    std::fs::write(gecici, kok.to_string()).map_err(|hata| format!("geçici dosya: {hata}"))?;
    std::fs::rename(gecici, VERI_YOLU).map_err(|hata| format!("veri.json: {hata}"))?;
    Ok(slug)
}

'''
p = 'src/web_veri.rs'
s = oku(p)
if 'pub fn tek_site_ekle' in s:
    print("atlandi: tek_site_ekle zaten var")
else:
    kanca = '#[cfg(test)]\nmod testler {'
    if kanca not in s:
        sys.exit("BULUNAMADI: web_veri test modulu")
    s = s.replace(kanca, TEK_SITE + kanca, 1)
    yaz(p, s)
    print("degisti: web_veri.rs tek_site_ekle")

# ---------------------------------------------------------------------------
# 2) katmanlar.rs — [2] akışı yayındaki siteleri silmeden tek sayfa ekler
# ---------------------------------------------------------------------------
p = 'src/katmanlar.rs'
s = oku(p)
bas = s.find("\nimpl LeadApp {\n    /// [2]'de çözülen işletmeyi keşif listesine katar")
if bas == -1:
    sys.exit("BULUNAMADI: seo_isletmesini_ekle impl blogu")
YENI = '''
impl LeadApp {
    /// [2]'de çözülen işletmenin sayfasını yayındaki listeye ekler: keşif
    /// taramasının ürettiği siteler korunur, yeni sayfa [3]'te önizlenir.
    fn seo_isletmesini_ekle(&mut self) {
        let Some(konum) = self.seo_konum.clone() else {
            self.seo_calistir();
            return;
        };
        let (il, ilce) = self.seo_konumu();
        let sektor = crate::isletme_linki::sektor_tahmini(&konum.ad);
        let kaynak = if konum.place_id.is_empty() {
            "https://www.google.com/maps".to_string()
        } else {
            format!(
                "https://www.google.com/maps/place/?q=place_id:{}",
                konum.place_id
            )
        };
        let lead = crate::BusinessLead {
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
        };
        match crate::web_veri::tek_site_ekle(&lead, &il, &ilce) {
            Ok(slug) => self.logs.push(format!(
                "[✓] {} · {slug} · meta/JSON-LD üretildi · sitemap tazelendi",
                konum.ad
            )),
            Err(hata) => self.logs.push(format!("[!] SEO sayfası yazılamadı: {hata}")),
        }
    }
}
'''
s = s[:bas].rstrip() + "\n" + YENI
yaz(p, s)
print("degisti: katmanlar.rs seo_isletmesini_ekle")

# ---------------------------------------------------------------------------
# 3) main.rs — gerçek paylaşım linki için uçtan uca test (elle, ağ ister)
# ---------------------------------------------------------------------------
TEST = '''
    /// Kullanıcının kendi paylaşım linki: yönlendirme → Places ad araması →
    /// SEO sayfası → yayın dosyası. Elle çalıştırılır, 1 Places isteği harcar:
    /// `cargo test --release share_linki_uctan_uca -- --ignored --nocapture`
    #[test]
    #[ignore = "canlı Google paylaşım linki + Places isteği harcar"]
    fn share_linki_uctan_uca() {
        let link = "https://share.google/RSn6csdDHU9pSBDIn";
        let konum = crate::isletme_linki::konum_coz(link).expect("link çözülmeli");
        assert!(!konum.ad.is_empty(), "işletme adı okunmalı");
        assert!((-90.0..=90.0).contains(&konum.enlem), "enlem geçerli");
        assert!((-180.0..=180.0).contains(&konum.boylam), "boylam geçerli");
        println!(
            "konum: {} · {} · place_id: {}",
            konum.ad,
            konum.koordinat(),
            konum.place_id
        );

        let sektor = crate::isletme_linki::sektor_tahmini(&konum.ad);
        let lead = BusinessLead {
            name: konum.ad.clone(),
            address: String::new(),
            category: sektor.clone(),
            phone: String::new(),
            selected: true,
            kaynak: format!(
                "https://www.google.com/maps/place/?q=place_id:{}",
                konum.place_id
            ),
            konum: Some((konum.enlem, konum.boylam)),
            calisma_saatleri: String::new(),
            demo_yolu: None,
            mesaj_hazir: false,
            kayit_dolulugu: 0,
            site_linki: String::new(),
            son_guncelleme_gun: None,
            inceleme_sayisi: None,
            yildiz_puani: None,
            durum: musteriler::Durum::Yeni,
            yayin_linki: None,
            karne_linki: None,
            mesaj_varyanti: None,
            foto: None,
            fotolar: Vec::new(),
            medya: Vec::new(),
        };
        let onceki = std::fs::read_to_string(crate::web_veri::VERI_YOLU)
            .ok()
            .and_then(|metin| serde_json::from_str::<serde_json::Value>(&metin).ok())
            .and_then(|veri| veri["siteler"].as_array().map(|s| s.len()))
            .unwrap_or(0);
        let slug = crate::web_veri::tek_site_ekle(&lead, "Manisa", "Şehzadeler").expect("sayfa");
        let metin = std::fs::read_to_string(crate::web_veri::VERI_YOLU).expect("veri.json");
        let veri: serde_json::Value = serde_json::from_str(&metin).expect("json");
        let siteler = veri["siteler"].as_array().expect("siteler");
        assert!(
            siteler.len() >= onceki.max(1),
            "yayındaki sayfalar silinmemeli ({onceki} → {})",
            siteler.len()
        );
        let site = siteler
            .iter()
            .find(|site| site["slug"].as_str() == Some(slug.as_str()))
            .expect("yeni sayfa listede olmalı")
            .clone();
        let baslik = site["baslik"].as_str().unwrap_or("");
        let aciklama = site["aciklama"].as_str().unwrap_or("");
        println!("sayfa: {slug} · title: {baslik} · desc: {aciklama}");
        assert!(!baslik.is_empty() && baslik.chars().count() <= 60);
        assert!(!aciklama.is_empty() && aciklama.chars().count() <= 160);
        assert!(!site["sema"].as_str().unwrap_or("").is_empty());
        assert_eq!(site["ilce"].as_str().unwrap_or(""), "Şehzadeler");
    }
'''
p = 'src/main.rs'
s = oku(p)
if 'fn share_linki_uctan_uca' in s:
    print("atlandi: share_linki_uctan_uca zaten var")
else:
    i = s.rstrip().rfind('\n}')
    if i == -1:
        sys.exit("BULUNAMADI: main.rs test modulu sonu")
    s = s[:i] + '\n' + TEST + s[i:]
    yaz(p, s)
    print("degisti: main.rs share_linki_uctan_uca testi")
print("TAMAM")
