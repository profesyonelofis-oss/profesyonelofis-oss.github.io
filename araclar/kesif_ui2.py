"""Site sınıflandırmasının kalıntılarını temizler.

Liste artık yalnızca web sitesi olmayan işletmelerden oluştuğu için
"Sosyal/Ucretsiz/Yok" ayrımı puanlama, demo metni ve karne kodundan kalkar.
"""

from pathlib import Path

YOL = Path("src/main.rs")
s = YOL.read_text(encoding="utf-8")


def rep(eski: str, yeni: str, adet: int = 1, ad: str = "") -> None:
    global s
    c = s.count(eski)
    assert c == adet, f"{ad}: beklenen {adet}, bulunan {c}"
    s = s.replace(eski, yeni)


# --- puanlama --------------------------------------------------------------
rep(
    """///   web varlığı: sosyal medya var ama site yok  +38
///                ücretsiz site aracı            +28
///                hiçbir web varlığı yok         +12
///   (niyet, ulaşılabilirlikten daha ağır basar: çevrimiçi görünmek isteyip
///    site yaptırmamış işletme, sitesiz ve iletişimsiz işletmeden öndedir.)
""",
    """///   web varlığı: hiçbir web sitesi yok          +12
///   (liste yalnızca sitesiz işletmelerden oluştuğu için bu puan sabittir;
///    sıralamada niyet sinyali olarak diğer ölçütlere eşlik eder.)
""",
    ad="puan belgesi",
)
rep(
    """    let mut puan: u16 = match isletme.site {
        // Çevrimiçi görünmek istediğini göstermiş ama site yaptırmamış.
        SiteDurumu::Sosyal => 38,
        // Site ihtiyacını kabul etmiş, ücretsiz araçla idare ediyor.
        SiteDurumu::Ucretsiz => 28,
        // Henüz web ihtiyacını fark etmemiş olabilir.
        SiteDurumu::Yok => 12,
    };
""",
    """    // Listeye yalnızca web sitesi olmayan işletmeler girer.
    let mut puan: u16 = 12;
""",
    ad="puan tabanı",
)

# --- gerekçe metni ---------------------------------------------------------
rep(
    """    let mut parcalar: Vec<String> = Vec::new();
    match isletme.site {
        SiteDurumu::Sosyal => parcalar.push("sosyal medya var, site yok".to_string()),
        SiteDurumu::Ucretsiz => parcalar.push("ücretsiz site aracı".to_string()),
        SiteDurumu::Yok => {}
    }
""",
    """    let mut parcalar: Vec<String> = Vec::new();
""",
    ad="gerekçe metni",
)

# --- WhatsApp açılış cümlesi ----------------------------------------------
rep(
    """    let site = match isletme.site {
        SiteDurumu::Yok => "web siteniz yok",
        SiteDurumu::Sosyal => "sosyal medya dışında siteniz yok",
        SiteDurumu::Ucretsiz => "kendi alan adınızda siteniz yok",
    };
""",
    """    // Listeye yalnızca sitesiz işletmeler girdiği için cümle sabittir.
    let site = "web siteniz yok";
""",
    ad="whatsapp cümlesi",
)

# --- dijital karne --------------------------------------------------------
rep(
    """    let (site_degeri, site_kayip, site_oneri) = match isletme.site {
        SiteDurumu::Yok => (
            "Web sitesi yok · sosyal medya hesabı da bulunamadı".to_string(),
            40,
            "Kendi alan adınızda mobil uyumlu bir site açın; Google profilinize ekleyin.",
        ),
        SiteDurumu::Sosyal => (
            format!("Web sitesi yok, yalnızca sosyal medya hesabı var{link}"),
            30,
            "Sosyal medya profiline ek olarak kendi alan adınızda bir site açın.",
        ),
        SiteDurumu::Ucretsiz => (
            format!("Web sitesi yok, ücretsiz site aracıyla kurulmuş sayfa var{link}"),
            20,
            "Ücretsiz alt alan adındaki sayfayı kendi alan adınıza taşıyın.",
        ),
    };
""",
    """    let site_degeri = if link.is_empty() {
        "Web sitesi yok · sosyal medya hesabı da bulunamadı".to_string()
    } else {
        format!("Web sitesi yok, kayıtlı adres{link}")
    };
    let site_kayip = 40;
    let site_oneri = "Kendi alan adınızda mobil uyumlu bir site açın; Google profilinize ekleyin.";
""",
    ad="karne site satırı",
)

# --- test kurulumları -----------------------------------------------------
rep(
    """            kayit_dolulugu: 0,
            site: SiteDurumu::Yok,
            site_linki: String::new(),
""",
    """            kayit_dolulugu: 0,
            site_linki: String::new(),
""",
    ad="test kurulumu 1",
)
rep(
    """                kayit_dolulugu: 0,
                site: SiteDurumu::Yok,
                site_linki: String::new(),
""",
    """                kayit_dolulugu: 0,
                site_linki: String::new(),
""",
    ad="test kurulumu 2",
)
rep(
    """        let mut sosyal = ornek_isletme();
        sosyal.address = "Barbaros Bulvarı No:12, Beşiktaş / İstanbul".to_string();
        sosyal.site = SiteDurumu::Sosyal;
        sosyal.site_linki = "https://instagram.com/kafe".to_string();
        assert_eq!(
            potansiyel_puani(&sosyal),
            100,
            "sosyal medyası olup sitesi olmayan en sıcak hedef (100'de tavanlanır)"
        );
        assert!(potansiyel_puani(&sosyal) > potansiyel_puani(&tam));
        assert!(potansiyel_gerekcesi(&sosyal).contains("sosyal medya var, site yok"));

        let mut incelemeli = ornek_isletme();
        incelemeli.site = SiteDurumu::Sosyal;
        incelemeli.inceleme_sayisi = Some(150);
""",
    """        let mut incelemeli = ornek_isletme();
        incelemeli.inceleme_sayisi = Some(150);
""",
    ad="puan testi",
)

# site türü testi tamamen kalkar (özellik yok)
bas = s.index("    #[test]\n    fn sosyal_medya_ve_ucretsiz_site_ayirt_edilir() {")
son = s.index("    #[test]\n    fn demo_renkleri_degistirilir_icerik_korunur() {")
s = s[:bas] + s[son:]

for kalinti in ["SiteDurumu", "site_durumu_bul", "url_alan_adi", "site_kodu", ".site'", "isletme.site"]:
    for no, satir in enumerate(s.split("\n"), 1):
        if kalinti in satir:
            print(f"KALINTI {kalinti}:{no}: {satir.strip()}")

YOL.write_text(s, encoding="utf-8", newline="")
print(f"tamam · {len(s)} bayt")
