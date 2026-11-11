//! LocalBusiness JSON-LD ve künye (SEO) yardımcıları.
//!
//! Temiz derlemede müşteri sayfasının `<head>` kısmına işletmenin gerçek
//! künyesini `application/ld+json` olarak basar: ad, adres, telefon ve
//! koordinat. Değerler uydurulmaz — kayıtta yoksa alan hiç yazılmaz.

use serde_json::{json, Value};

/// JSON-LD'ye girecek işletme künyesi.
pub struct IsletmeVerisi<'a> {
    pub ad: &'a str,
    pub adres: &'a str,
    pub telefon: &'a str,
    pub sektor: &'a str,
    pub konum: Option<(f64, f64)>,
    /// Yayındaki adres (varsa `url` alanına yazılır).
    pub url: Option<&'a str>,
}

/// Telefonu E.164 biçimine çevirir: "0507 12 99 439" → "+905071299439".
/// Sabit hat/cep ayrımı yapılmaz; 10-11 haneli numara kabul edilir.
pub fn telefon_e164(telefon: &str) -> Option<String> {
    let rakamlar: String = telefon.chars().filter(char::is_ascii_digit).collect();
    let govde = match rakamlar.len() {
        11 if rakamlar.starts_with('0') => rakamlar[1..].to_string(),
        10 => rakamlar,
        12 if rakamlar.starts_with("90") => rakamlar[2..].to_string(),
        13 if rakamlar.starts_with("+90") => rakamlar[3..].to_string(),
        _ => return None,
    };
    if govde.len() != 10 {
        return None;
    }
    Some(format!("+90{govde}"))
}

/// Adresi sokak ve şehir olarak ikiye ayırır: son virgüllü parça şehir sayılır.
pub fn adres_parcala(adres: &str) -> (String, Option<String>) {
    let adres = adres.trim();
    match adres.rsplit_once(',') {
        Some((sokak, sehir)) if !sokak.trim().is_empty() && !sehir.trim().is_empty() => {
            (sokak.trim().to_string(), Some(sehir.trim().to_string()))
        }
        _ => (adres.to_string(), None),
    }
}

/// JSON-LD gövdesi (script etiketi olmadan).
pub fn json_ld_govdesi(v: &IsletmeVerisi) -> Value {
    let (sokak, sehir) = adres_parcala(v.adres);
    let mut adres_alani = json!({
        "@type": "PostalAddress",
        "streetAddress": sokak,
        "addressCountry": "TR",
    });
    if let Some(sehir) = sehir {
        adres_alani["addressLocality"] = Value::String(sehir);
    }
    let mut govde = json!({
        "@context": "https://schema.org",
        "@type": "LocalBusiness",
        "name": v.ad,
        "description": format!("{} — {}", v.ad, v.sektor),
        "address": adres_alani,
    });
    if let Some(telefon) = telefon_e164(v.telefon) {
        govde["telephone"] = Value::String(telefon);
    }
    if let Some((enlem, boylam)) = v.konum {
        govde["geo"] = json!({
            "@type": "GeoCoordinates",
            "latitude": enlem,
            "longitude": boylam,
        });
    }
    if let Some(url) = v.url.filter(|url| !url.trim().is_empty()) {
        govde["url"] = Value::String(url.to_string());
    }
    govde
}

/// `<script type="application/ld+json">…</script>` bloğu.
pub fn json_ld(v: &IsletmeVerisi) -> String {
    let govde = json_ld_govdesi(v).to_string();
    // "</" dizisi sayfayı erken kapatmasın diye kaçırılır (JSON'da geçerli).
    let guvenli = govde.replace("</", "<\\/");
    format!("<script type=\"application/ld+json\">{guvenli}</script>\n")
}

/// Bloğu sayfanın `<head>` kısmının sonuna ekler. Sayfada zaten JSON-LD varsa
/// ikinci kez eklenmez (yeniden derlemede çoğalmaz).
pub fn sayfaya_ekle(html: &str, v: &IsletmeVerisi) -> String {
    if html.contains("application/ld+json") {
        return html.to_string();
    }
    let blok = json_ld(v);
    match html.find("</head>") {
        Some(konum) => {
            let mut yeni = String::with_capacity(html.len() + blok.len());
            yeni.push_str(&html[..konum]);
            yeni.push_str(&blok);
            yeni.push_str(&html[konum..]);
            yeni
        }
        None => format!("{blok}{html}"),
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn veri() -> IsletmeVerisi<'static> {
        IsletmeVerisi {
            ad: "Saruhanlı Oto Kurtarma",
            adres: "Cumhuriyet Mah. Sanayi Cad. No:12, Saruhanlı",
            telefon: "0507 12 99 439",
            sektor: "Çekici",
            konum: Some((38.7342, 27.5658)),
            url: Some("https://ornek.github.io/demos/saruhanli.html"),
        }
    }

    #[test]
    fn json_ld_alanlari_icerir() {
        let govde = json_ld_govdesi(&veri());
        assert_eq!(govde["@context"], "https://schema.org");
        assert_eq!(govde["@type"], "LocalBusiness");
        assert_eq!(govde["name"], "Saruhanlı Oto Kurtarma");
        assert_eq!(govde["telephone"], "+905071299439");
        assert_eq!(govde["geo"]["latitude"], 38.7342);
        assert_eq!(govde["geo"]["longitude"], 27.5658);
        assert_eq!(govde["address"]["addressLocality"], "Saruhanlı");
        assert_eq!(govde["address"]["addressCountry"], "TR");
        assert!(govde["address"]["streetAddress"]
            .as_str()
            .unwrap_or_default()
            .contains("Sanayi Cad."));
        assert_eq!(govde["url"], "https://ornek.github.io/demos/saruhanli.html");
    }

    #[test]
    fn eksik_veri_uydurulmaz() {
        let sade = IsletmeVerisi {
            ad: "Dükkan",
            adres: "",
            telefon: "",
            sektor: "Kuaför",
            konum: None,
            url: None,
        };
        let govde = json_ld_govdesi(&sade);
        assert!(govde.get("telephone").is_none());
        assert!(govde.get("geo").is_none());
        assert!(govde.get("url").is_none());
    }

    #[test]
    fn telefon_uluslararasi_bicime_cevrilir() {
        assert_eq!(
            telefon_e164("0507 12 99 439").as_deref(),
            Some("+905071299439")
        );
        assert_eq!(
            telefon_e164("+90 507 129 94 39").as_deref(),
            Some("+905071299439")
        );
        assert_eq!(telefon_e164("5071299439").as_deref(), Some("+905071299439"));
        assert_eq!(telefon_e164("123"), None);
        assert_eq!(telefon_e164(""), None);
    }

    #[test]
    fn blok_head_icine_bir_kez_eklenir() {
        let ham = "<html><head><title>X</title></head><body>içerik</body></html>";
        let bir = sayfaya_ekle(ham, &veri());
        assert_eq!(bir.matches("application/ld+json").count(), 1);
        // Blok gerçekten <head> içinde ve </head>'den önce olmalı.
        let blok = bir.find("application/ld+json").unwrap_or(usize::MAX);
        let kapanis = bir.find("</head>").unwrap_or(0);
        assert!(blok < kapanis, "JSON-LD head içinde değil");
        assert!(bir.contains(
            "</script>
</head>"
        ));
        let iki = sayfaya_ekle(&bir, &veri());
        assert_eq!(iki.matches("application/ld+json").count(), 1);
        assert_eq!(bir, iki);
    }

    #[test]
    fn script_etiketi_erken_kapatilmaz() {
        let kotu = IsletmeVerisi {
            ad: "Kapan </script><script>alert(1)</script>",
            adres: "Adres",
            telefon: "05071234567",
            sektor: "Test",
            konum: None,
            url: None,
        };
        let blok = json_ld(&kotu);
        assert_eq!(blok.matches("</script>").count(), 1);
        assert!(blok.contains("<\\/script>"));
    }

    #[test]
    fn adres_parcalanir() {
        let (sokak, sehir) = adres_parcala("Cumhuriyet Mah. No:1, Manisa");
        assert_eq!(sokak, "Cumhuriyet Mah. No:1");
        assert_eq!(sehir.as_deref(), Some("Manisa"));
        let (sokak, sehir) = adres_parcala("Tek parça adres");
        assert_eq!(sokak, "Tek parça adres");
        assert!(sehir.is_none());
    }
}
