//! Demo sitesi üreticisi.
//!
//! Uygulamanın "Demo Üret" akışı bu modülü kullanır: işletme kaydındaki gerçek
//! verileri (ad, adres, telefon, çalışma saatleri, konum, puan) sektöre özel
//! metinlerle birleştirip tek dosyalık, bağımsız bir tanıtım sayfası üretir.
//!
//! Tasarım kuralları:
//! - Dış istek yok: CSS/JS/fontlar sayfanın içinde (CDN, internet gerekmez).
//! - Tailwind gibi bir çalışma zamanı yok; ölçü ve boşluklar sabit ızgarada.
//! - Hareket; satır maskesi, kaydırma açılımı, parallax, akan şerit ve
//!   imleci izleyen ışıkla sınırlı — `prefers-reduced-motion` hepsini kapatır.
//! - İşletme hakkında uydurma iddia yok: puan/yorum yalnızca kayıtta varsa,
//!   fiyat listesi bilinçli olarak yok, yorumların örnek olduğu yazar.

use crate::tasarim_brifi::Brif;

/// Sayfada hangi bölümlerin görüneceği. Panelden açılıp kapatılır;
/// varsayılan hepsi açık.
#[derive(Clone, Copy, Debug)]
pub struct Bolumler {
    pub galeri: bool,
    pub yorumlar: bool,
    pub harita: bool,
    pub saatler: bool,
}

impl Default for Bolumler {
    fn default() -> Self {
        Self {
            galeri: true,
            yorumlar: true,
            harita: true,
            saatler: true,
        }
    }
}

/// Panelden yazılan metinler. Boş alan sektör şablonundan gelir; böylece
/// yalnız başlığı değiştirmek isteyen kullanıcı onlarca alanı doldurmaz.
#[derive(Clone, Debug, Default)]
pub struct Metinler {
    pub manset1: String,
    pub manset2: String,
    pub hakkinda_baslik: String,
    pub hakkinda1: String,
    pub hakkinda2: String,
    pub bolum_etiket: String,
    pub bolum_baslik: String,
    pub bolum_alt: String,
    /// Bölüm listesindeki dört satır: (ad, açıklama).
    pub kalemler: Vec<(String, String)>,
    /// İki yorum: (söz, isim).
    pub yorumlar: Vec<(String, String)>,
}

impl Metinler {
    /// Listenin `sira`. satırındaki alan; satır yoksa boş.
    fn satir<'a>(&self, liste: &'a [(String, String)], sira: usize, alan: usize) -> &'a str {
        liste
            .get(sira)
            .map(|(ad, aciklama)| if alan == 0 { ad } else { aciklama })
            .map(String::as_str)
            .unwrap_or("")
    }
}

/// Sayfayı üretmek için gereken her şey; `main.rs` hazırlayıp buraya verir.
/// Sayfaya gömülen fotoğraf: (mime, base64).
pub type FotoVerisi = (String, String);

/// Sektörel tasarım hafızasından gelen kimlik. Boş alan şablon varsayılanına
/// düşer; panelden yazılan metin her zaman önceliklidir.
#[derive(Clone, Debug, Default)]
pub struct TasarimGirdi {
    pub baslik: String,
    pub govde: String,
    pub aciliyet: u8,
    pub sinematik: bool,
    pub rozetler: Vec<String>,
    pub cta: String,
    pub vurgu2: String,
    pub ad: String,
    pub grup: String,
    pub hareket: String,
}

/// Sinematik profil ayarları: panelden gelen üç seçim. Bilinmeyen değer
/// güvenli varsayılana düşer; eski ayar dosyaları bu alanı hiç taşımaz.
#[derive(Clone, Debug)]
pub struct SinematikAyar {
    /// "dusuk" · "orta" · "yuksek"
    pub hareket: String,
    /// "otomatik" veya "halka" · "kup" · "dalga" · "geometri"
    pub nesne: String,
    /// Sahne sayısı: 3–6 arasına kırpılır.
    pub sahne: usize,
}

impl Default for SinematikAyar {
    fn default() -> Self {
        Self {
            hareket: "orta".to_string(),
            nesne: "otomatik".to_string(),
            sahne: 4,
        }
    }
}

/// Hazırlanmış sinematik hero videosu: dosya ve poster sayfaya göreli
/// (`media/hero.mp4`), atıf yalnız lisans istiyorsa dolu gelir.
#[derive(Clone, Debug, Default)]
pub struct VideoGirdi {
    pub dosya: String,
    pub poster: String,
    pub atif: Option<String>,
}

#[derive(Clone)]
pub struct Girdi {
    pub ad: String,
    pub adres: String,
    pub sektor: String,
    /// Sektörün demo ailesi: "menu", "kafe", "unlu", "urun", "galeri",
    /// "klinik", "otel", "egitim", "ofis", "servis", "eglence".
    pub tur: &'static str,
    /// Vurgu rengi (#rrggbb). Boşsa sektör rengi kullanılır.
    pub vurgu: String,
    /// Sayfa zemini ve metin rengi (#rrggbb): uygulamadaki renklendirme paneli.
    pub zemin: String,
    pub yazi: String,
    /// İşletmenin fotoğrafı: varsa hero sahnesinde kullanılır.
    pub foto: Option<FotoVerisi>,
    /// Fotoğrafın atfı (lisans gereği): altbilgide görünür.
    pub foto_atif: Option<String>,
    /// İşletmenin ek fotoğrafları (mime, base64): galeri bölümü.
    /// Boşsa bölüm hiç çizilmez.
    pub galeri: Vec<FotoVerisi>,
    /// Galeri işletmenin kendi fotoğrafları mı, sektörün örnek kareleri mi?
    pub galeri_ornek: bool,
    /// Galeri kaynağının kısa açıklaması (ör. "işletmenin fotoğrafları ·
    /// sokağından"). Boşsa örnek/işletme fotoğrafına göre varsayılan yazılır.
    pub galeri_not: Option<String>,
    /// Örnek karelerin atıf satırı (lisans gereği); işletme fotoğrafında boş.
    pub galeri_atif: Option<String>,
    pub telefon: Option<String>,
    pub telefon_goster: String,
    /// Haritanın gömüleceği koordinat; yoksa harita bölümü çizilmez.
    pub konum: Option<(f64, f64)>,
    /// Haftalık program (JS dizisi); saat verisi yoksa `None`.
    pub program: Option<String>,
    pub saat_metni: String,
    pub saat_satirlari: Vec<(String, String)>,
    pub puan: Option<String>,
    pub yorum: Option<String>,
    pub karne: Option<String>,
    pub kaynak: String,
    pub gonderen: String,
    pub anahtar: String,
    /// Bölüm görünürlüğü: istenmeyen bölüm sayfadan tamamen çıkar.
    pub bolumler: Bolumler,
    /// Marka cümlesi: hero'nun altındaki tek cümle. Boşsa sektör metni.
    pub slogan: String,
    /// Birincil buton metni. Boşsa sektör metni (ör. "Masa ayırt").
    pub cta: String,
    /// Köşe biçimi: "keskin" veya "yumusak"; boşsa dengeli.
    pub kose: String,
    /// Yerleşim: boşsa klasik çok bölümlü sayfa; "gece" ise koyu, pirinç
    /// çizgili düzen: hero, hikâye, liste ve form'lu iletişim.
    pub tema: String,
    /// İkincil buton metni ("Konumu gör"/"İletişim"); boşsa otomatik.
    pub ikincil: String,
    /// Panelden yazılan başlık, paragraf, liste ve yorum metinleri.
    pub metin: Metinler,
    /// "Hakkında" bölümünün görseli (mime, base64). Galeriden ayrılan bir kare
    /// buraya gelir: aynı fotoğraf sayfada iki kez görünmez.
    pub hikaye_foto: Option<FotoVerisi>,
    /// Satıcının WhatsApp numarası, yalnız rakam (ör. "905321234567").
    /// Varsa sayfa sonunda "bu taslağı size özel yapalım" bandı çıkar;
    /// boşsa (numara tanımlı değilse) bant hiç çizilmez.
    pub satici_tel: Option<String>,
    pub font_css: &'static str,
    /// Sinematik profil seçimleri (hareket, 3D nesne, sahne sayısı).
    pub sinematik: SinematikAyar,
    /// Sinematik hero videosu: varsa kodla çizilen 3D nesne yerine video
    /// oynar (poster önce, video arkadan). Yoksa mevcut shader sürümü.
    pub video: Option<VideoGirdi>,
    /// İşletmenin medya tohumu: Ken Burns yönü, video başlangıcı ve paralaks
    /// derinliği her işletmede farklı olsun.
    pub medya_tohum: u64,
    /// Sektörün tasarım kimliği; `None` ise yalnız şablon varsayılanı.
    pub tasarim: Option<TasarimGirdi>,
    /// Üretim profili: "ajans" beş bölümlü sinematik akış, "klasik" eski şablon.
    pub profil: String,
    /// AI ajanının `tasarim_brifi.json` üzerinden verdiği tasarım sözleşmesi.
    pub brif: Option<Brif>,
}

struct Icerik {
    /// İkinci bölümün üst etiketi (başlığın kısa adı).
    etiket: &'static str,
    /// Manşet iki satıra bölünür; satırlar maskeden yukarı kayarak girer.
    manset: [&'static str; 2],
    alt: &'static str,
    /// Hakkında bölümünün başlığı.
    hikaye_baslik: &'static str,
    hikaye: [&'static str; 2],
    bolum: &'static str,
    bolum_alt: &'static str,
    /// (ad, açıklama) — fiyat bilinçli olarak yok; işletme fiyat
    /// vermeden de teklif götürülebilsin.
    kalemler: [(&'static str, &'static str); 4],
    neden: [(&'static str, &'static str); 3],
    cta: &'static str,
    akis: &'static str,
    yorumlar: [(&'static str, &'static str); 2],
}

fn icerik(tur: &str) -> Icerik {
    match tur {
        "menu" => Icerik {
            etiket: "MENÜ",
            manset: ["Ateşin ve sofranın", "ortak dili."],
            alt: "Günlük taze malzeme, meşe kömürü ve uzun sofralar. Mutfak her sabah yeniden kurulur.",
            hikaye_baslik: "Otuz yıldır aynı ocak.",
            hikaye: [
                "Ocağın başında geçen otuz yıl bize tek bir şeyi öğretti: iyi yemek acele etmez. Sebze sabah pazardan, et aynı gün kasaptan gelir; menü mevsime göre değişir.",
                "Sofraya oturduğunuzda garsonun ilk sorusu ne yiyeceğiniz değil, ne kadar kalacağınız. Acele ettirmiyoruz — burası hızlı yemek yeri değil.",
            ],
            bolum: "Menüden seçkiler",
            bolum_alt: "Mevsimlik menünün bir bölümü. Tam liste işletmede ve telefonda.",
            kalemler: [
                ("Günün çorbası", "Mevsim sebzeleri, tereyağı, taze nane"),
                ("Adana kebap", "Zırhla kıyılmış kuzu, közlenmiş biber, sumaklı soğan"),
                ("Kuzu tandır", "Sekiz saat kapalı fırında, kuyruk yağıyla"),
                ("Fıstıklı kadayıf", "Günlük açılan kadayıf, Antep fıstığı, kaymak"),
            ],
            neden: [
                ("Günlük tedarik", "Sebze ve et her sabah gelir; bir gün bile beklemez."),
                ("Odun ateşi", "Gaz değil meşe kömürü — kokusu tabağa siner."),
                ("Uzun sofra", "Kalabalık gruplar için ayrı salon; önceden ayırtılır."),
            ],
            cta: "Masa ayırt",
            akis: "Kebap · Meze · Rakı · Tatlı",
            yorumlar: [
                ("Tandır geldiğinde masa sustu. İki saat oturduk, kimse kaldırmadı.", "Akşam yemeği"),
                ("Rezervasyonu telefonla yaptık, saatinde masa hazırdı.", "Kalabalık grup"),
            ],
        },
        "kafe" => Icerik {
            etiket: "MENÜ",
            manset: ["Sabahın ilk fincanı,", "akşamın son sohbeti."],
            alt: "Üçüncü nesil çekirdek, haftalık kavurma. Sütü çevredeki üreticiden alıyoruz.",
            hikaye_baslik: "Haftada iki kez kavuruyoruz.",
            hikaye: [
                "Çekirdeği kendi mutfağımızda, haftada iki kez küçük partiler hâlinde kavuruyoruz. Demleme suyunun sertliğini ölçüyor, her yeni çekirdekte öğütmeyi yeniden ayarlıyoruz.",
                "Masa sizi bekletmez: priz var, wifi var, iki saat otursanız kimse bakmaz. Ama gürültü yok — konuşmak için geldiyseniz de doğru yerdesiniz.",
            ],
            bolum: "Menü",
            bolum_alt: "Çekirdekler mevsime göre değişir; barista önerir.",
            kalemler: [
                ("Filtre kahve", "Tek origin, V60, günün çekirdeği"),
                ("Flat white", "Çift shot, mikro köpük, 160 ml"),
                ("Cold brew", "18 saat soğuk demleme, buzla"),
                ("Ev yapımı cheesecake", "Günlük, fırın tepsisiyle sınırlı"),
            ],
            neden: [
                ("Kendi kavurmamız", "Çekirdek her hafta taze kavrulur, raf ömrü kısa tutulur."),
                ("Sessiz köşe", "Priz, wifi ve rahat oturma; uzun kalmak sorun değil."),
                ("Ev yapımı tatlı", "Tatlılar her sabah mutfakta hazırlanır, kalan ertesi güne çıkmaz."),
            ],
            cta: "Masa ayırt",
            akis: "Espresso · Filtre · Tatlı · Sohbet",
            yorumlar: [
                ("Filtre kahvesi şehirdeki en dengelisi; acı değil, meyvemsi.", "Sabah kahvesi"),
                ("Laptopla üç saat çalıştım, kimse rahatsız etmedi.", "Uzaktan çalışan"),
            ],
        },
        "unlu" => Icerik {
            etiket: "TEZGÂH",
            manset: ["Her sabah 05:00'te", "yanan ocak."],
            alt: "Ekşi mayalı ekmek, taze simit ve pastane tatlıları; hepsi aynı tezgâhta.",
            hikaye_baslik: "Maya 2011'den beri aynı kavanozda.",
            hikaye: [
                "Mayamız 2011'den beri aynı kavanozda. Un, su ve tuz; başka bir şey girmiyor. Hamur 36 saat dinleniyor, bu yüzden ekmek ertesi gün de taze kalıyor.",
                "Vitrinde ne varsa o gün üretildi. Kapanışa kalan ürünü ertesi güne çıkarmıyoruz — gün sonu indirimi bu yüzden var.",
            ],
            bolum: "Tezgâhtan",
            bolum_alt: "Günlük üretim; bazı ürünler öğleden sonra biter.",
            kalemler: [
                ("Ekşi mayalı somun", "36 saat fermentasyon, tam buğday katkılı"),
                ("Simit", "Susamlı, taş fırın"),
                ("Su böreği (porsiyon)", "Günlük açılmış yufka, peynirli"),
                ("Doğum günü pastası", "Siparişe göre, 24 saat önceden"),
            ],
            neden: [
                ("Ekşi maya", "Katkı maddesi yok; maya, un, su, tuz."),
                ("Günlük üretim", "Kalan ürün ertesi güne devredilmez."),
                ("Sipariş", "Pasta ve börek için 24 saat önce arayın."),
            ],
            cta: "Sipariş ver",
            akis: "Ekmek · Simit · Börek · Pasta",
            yorumlar: [
                ("Ekmeği iki gün sonra da kızartmadan yiyoruz.", "Haftalık ekmek"),
                ("Pastayı bir gün önceden söyledik, tam istediğimiz gibi geldi.", "Doğum günü"),
            ],
        },
        "urun" => Icerik {
            etiket: "SEÇKİ",
            manset: ["Seçilmiş ürün,", "sade sunum."],
            alt: "Her sezon az sayıda parça. Denemeden almaya gerek yok; değişim otuz gün.",
            hikaye_baslik: "Vitrine giren her parça denenir.",
            hikaye: [
                "Vitrine koyduğumuz her parçayı önce kendimiz kullanıyoruz. Dikişi, kumaşı, yıkandıktan sonraki hâli tutmuyorsa rafa girmiyor.",
                "Beden konusunda tahmin yürütmüyoruz: ölçü tablosu kapının yanında, deneme kabini var. Almadan çıkmak da sorun değil.",
            ],
            bolum: "Sezon seçkisi",
            bolum_alt: "Beden ve renk stokları mağazaya göre değişir.",
            kalemler: [
                ("Gömlek", "Pamuk poplin, regular kalıp, altı renk"),
                ("Pantolon", "Yüksek bel, %2 elastan, dört beden"),
                ("Triko", "Merinos karışım, ince örgü"),
                ("Aksesuar", "Kemer, atkı, çanta"),
            ],
            neden: [
                ("Sınırlı koleksiyon", "Her bedenden az adet; aynı parçayı herkeste görmezsiniz."),
                ("Otuz gün değişim", "Etiketi sökülmemiş ürün koşulsuz değişir."),
                ("Kombin yardımı", "Randevusuz; satıcı değil, danışman gibi çalışıyoruz."),
            ],
            cta: "Mağazayı ara",
            akis: "Giyim · Aksesuar · Beden · Değişim",
            yorumlar: [
                ("Bedenimi bilmeden gittim, on dakikada doğru kalıbı bulduk.", "İlk alışveriş"),
                ("Aldığım gömleği bir beden büyüğüyle sorunsuz değiştirdiler.", "Değişim"),
            ],
        },
        "galeri" => Icerik {
            etiket: "HİZMETLER",
            manset: ["Aynaya bakmadan", "çıkmıyorsunuz."],
            alt: "Kesim, renk ve bakım saçınızın yapısına göre planlanır; şablon yok.",
            hikaye_baslik: "İlk on dakika makas açılmıyor.",
            hikaye: [
                "İlk on dakika makas açılmıyor: saçın ne kadar yıprandığını, ne sıklıkla şekil verdiğinizi, sabah ne kadar vaktiniz olduğunu konuşuyoruz. Plan buna göre çıkıyor.",
                "Boyada tek marka kullanmıyoruz; saçın direncine göre seçiyoruz. Kullandığımız ürünü sormak ayıp değil — hepsini söylüyoruz.",
            ],
            bolum: "Hizmetler",
            bolum_alt: "Süre ve kapsam saçın yapısına göre planlanır; telefonda netleşir.",
            kalemler: [
                ("Saç kesimi", "Yıkama ve fön dahil, 45 dakika"),
                ("Boya & röfle", "Saç analizine göre teknik, 2–3 saat"),
                ("Keratin bakım", "Yıpranmış saç için, 2 saat"),
                ("Gelin saçı", "Deneme seansı dahil"),
            ],
            neden: [
                ("Konsültasyon", "İlk on dakika sadece dinliyoruz."),
                ("Randevu disiplini", "Saatler 15 dakikalık aralıklarla; bekleme yok."),
                ("Şeffaf ürün", "Hangi marka, hangi oran — sorduğunuzda söylüyoruz."),
            ],
            cta: "Randevu al",
            akis: "Kesim · Renk · Bakım · Gelin",
            yorumlar: [
                ("Yıllardır boya yaptırıyorum, ilk kez saçım yıpranmadı.", "Renk"),
                ("Randevu saatinde başladı, söylenen sürede bitti.", "Kesim"),
            ],
        },
        "klinik" => Icerik {
            etiket: "TEDAVİLER",
            manset: ["Tedavi planı,", "ilk muayenede."],
            alt: "Dijital görüntüleme, tek kullanımlık setler ve yazılı tedavi planı.",
            hikaye_baslik: "Plan, ilk muayenede çıkar.",
            hikaye: [
                "İlk muayenede ağzınızın tamamını görüyoruz: röntgen ve ağız içi tarama ile. Sonra tek tek ne yapılacağını, kaç seansta biteceğini ve ne tutacağını yazılı veriyoruz.",
                "Ağrınız varsa aynı gün bakıyoruz. Tedaviye başlamadan önce alternatifleri de anlatıyoruz — en pahalı seçenek her zaman doğru seçenek değil.",
            ],
            bolum: "Tedaviler",
            bolum_alt: "Tedavi planı muayeneden sonra çıkar.",
            kalemler: [
                ("İlk muayene & röntgen", "Panoramik film ve ağız içi tarama dahil"),
                ("Diş temizliği", "Ultrasonik temizlik, 30 dakika"),
                ("Beyazlatma", "Ofis tipi, tek seans"),
                ("İmplant", "Tek diş, kemik grefti hariç"),
            ],
            neden: [
                ("Sterilizasyon", "Her set tek kullanımlık; otoklav kaydı istenebilir."),
                ("Dijital ölçü", "Ağız içi tarayıcı — ölçü macunu yok."),
                ("Yazılı plan", "Seans sayısı ve tutar, başlamadan önce yazılı."),
            ],
            cta: "Randevu al",
            akis: "Muayene · Temizlik · İmplant · Estetik",
            yorumlar: [
                ("Ne yapılacağını ve ne ödeyeceğimi baştan yazılı gördüm.", "İmplant"),
                ("Diş hekimi korkum vardı; ilk seansta bunu konuştuk.", "Tedavi"),
            ],
        },
        "otel" => Icerik {
            etiket: "ODALAR",
            manset: ["Şehirde ama", "gürültüden uzak."],
            alt: "On iki oda, kahvaltı dahil. Her oda farklı; hiçbiri standart değil.",
            hikaye_baslik: "On iki oda, on iki farklı plan.",
            hikaye: [
                "Bina 1920'lerden; odaları tek tek elden geçirdik. Yüksek tavan, ahşap zemin ve sokağa bakan pencereler kaldı, gerisi yenilendi.",
                "Kahvaltı açık büfe değil, masaya geliyor: yerel peynir, zeytin, günlük ekmek. Erken çıkmanız gerekiyorsa bir gece önce söyleyin, paket yaparız.",
            ],
            bolum: "Odalar",
            bolum_alt: "Odalar sezona göre değişir; müsaitlik rezervasyonda netleşir.",
            kalemler: [
                ("Standart oda", "18 m², çift kişilik, kahvaltı dahil"),
                ("Deluxe oda", "26 m², balkon, kahvaltı dahil"),
                ("Suit", "38 m², oturma odası ayrı"),
                ("Kahvaltı (misafir dışı)", "08:00–10:30, açık büfe"),
            ],
            neden: [
                ("Kahvaltı dahil", "Yerel ürünlerle, 08:00–10:30 arası."),
                ("Yürüme mesafesi", "Tarihi merkez ve sahil on dakika."),
                ("Esnek iptal", "Girişten 48 saat öncesine kadar ücretsiz."),
            ],
            cta: "Rezervasyon",
            akis: "Oda · Kahvaltı · Konum · İptal",
            yorumlar: [
                ("Odalar fotoğraftakinden büyük; sessizlik en büyük artısı.", "Hafta sonu"),
                ("Kahvaltı masaya geldi, otel büfesi gibi değil.", "Konaklama"),
            ],
        },
        "egitim" => Icerik {
            etiket: "PROGRAMLAR",
            manset: ["Küçük sınıf,", "çok konuşma."],
            alt: "Sekiz kişilik gruplar, ücretsiz seviye sınavı ve haftada iki konuşma seansı.",
            hikaye_baslik: "Sekiz kişilik sınıflar.",
            hikaye: [
                "Sınıflar en fazla sekiz kişi. Seviye sınavı kayıttan önce yapılır; kimse yanlış gruba başlamaz. Ay sonunda ilerleme yazılı olarak paylaşılır.",
                "Dilbilgisi tek başına konuşmayı getirmiyor; bu yüzden haftada iki seans yalnızca konuşmaya ayrıldı. Hata yapmak serbest, susmak değil.",
            ],
            bolum: "Programlar",
            bolum_alt: "Gruplar ayda bir açılır; kontenjan sekiz kişi.",
            kalemler: [
                ("Genel İngilizce", "A1–C1, haftada 2 gün, 8 hafta"),
                ("Sınav hazırlığı", "IELTS / YDS odaklı, 10 hafta"),
                ("Birebir ders", "Öğretmenle teke tek, 60 dakika"),
                ("Çocuk grubu", "8–12 yaş, haftada 2 gün"),
            ],
            neden: [
                ("Sekiz kişi", "Sınıf mevcudu sekizle sınırlı."),
                ("Konuşma odağı", "Haftada iki seans yalnızca konuşma."),
                ("Ücretsiz seviye testi", "Kayıttan önce, sonuç yazılı."),
            ],
            cta: "Seviye testi",
            akis: "İngilizce · Sınav · Konuşma · Çocuk",
            yorumlar: [
                ("Altı ay önce konuşamıyordum, şimdi toplantıda konuşuyorum.", "Genel İngilizce"),
                ("Çocuk grubunda hocayı sevdi, devam etmek istiyor.", "Çocuk grubu"),
            ],
        },
        "ofis" => Icerik {
            etiket: "HİZMETLER",
            manset: ["İlk görüşmede", "net bir yol haritası."],
            alt: "Dosyanızı dinliyor, süreci ve maliyeti yazılı olarak önünüze koyuyoruz.",
            hikaye_baslik: "Belirsizliği yazıya döküyoruz.",
            hikaye: [
                "Danışmanlıkta en büyük sorun belirsizlik: ne olacağını, ne kadar süreceğini ve ne tutacağını bilmemek. İlk görüşmede bunların hepsini kâğıda döküyoruz.",
                "Dosyanızı bir ekip değil, tek bir sorumlu yürütür. Aradığınızda kimi aradığınızı bilirsiniz; her seferinde baştan anlatmanız gerekmez.",
            ],
            bolum: "Hizmetler",
            bolum_alt: "Kapsam, ilk görüşmeden sonra yazılı teklifle netleşir.",
            kalemler: [
                ("İlk görüşme", "30 dakika, dosya değerlendirmesi"),
                ("Danışmanlık", "Saatlik, yazılı özet dahil"),
                ("Sözleşme incelemesi", "Risk notlarıyla birlikte"),
                ("Aylık takip", "Süreç ve yazışma takibi"),
            ],
            neden: [
                ("Yazılı kapsam", "Ne yapılacağı ve süresi baştan yazılır."),
                ("Gizlilik", "Dosyalar yalnızca sorumlu uzmanda; üçüncü tarafla paylaşılmaz."),
                ("Aynı gün dönüş", "Mesaj ve e-postaya aynı iş günü içinde dönülür."),
            ],
            cta: "Görüşme planla",
            akis: "Danışmanlık · Sözleşme · Takip · Gizlilik",
            yorumlar: [
                ("Ne yapılacağını ve ne ödeyeceğimi ilk görüşmede yazılı aldım.", "Danışmanlık"),
                ("Süreç boyunca aynı kişiyle konuştum, her seferinde baştan anlatmadım.", "Dosya takibi"),
            ],
        },
        "servis" => Icerik {
            etiket: "HİZMETLER",
            manset: ["Yolda kaldıysanız,", "ortalama 25 dakika."],
            alt: "7/24 çekici, akü takviye ve lastik. Konumunuzu gönderin, en yakın aracı yönlendirelim.",
            hikaye_baslik: "Yolda bırakmayan ekip.",
            hikaye: [
                "Çağrı merkezi değil, doğrudan sürücüyle konuşuyorsunuz. Konumunuzu attığınızda en yakın araç yola çıkıyor ve tahmini varış süresini mesajla alıyorsunuz.",
                "İşe başlamadan kapsamı telefonda konuşuyoruz. Aracınızı nereye götürdüğümüzü ve teslim saatini de yine mesajla bildiriyoruz.",
            ],
            bolum: "Hizmetler",
            bolum_alt: "Şehir içi ve şehirler arası hizmet; mesafe baştan konuşulur.",
            kalemler: [
                ("Şehir içi çekici", "İlk 15 km dahil"),
                ("Akü takviye", "Yerinde, 15 dakika"),
                ("Lastik değişimi", "Yedek lastik varsa"),
                ("Şehirler arası", "Kilometre başı, önceden hesaplanır"),
            ],
            neden: [
                ("7/24", "Gece, hafta sonu, resmî tatil fark etmez."),
                ("Açık kapsam", "İşe başlamadan ne yapılacağı konuşulur."),
                ("Takip", "Aracın nereye gittiğini mesajla bildiririz."),
            ],
            cta: "Hemen ara",
            akis: "Çekici · Akü · Lastik · 7/24",
            yorumlar: [
                ("Gece 02:00'de aradım, 20 dakikada geldiler.", "Gece çağrısı"),
                ("Telefonda anlatılanla yapılan iş birebir aynıydı.", "Şehir içi çekici"),
            ],
        },
        "eglence" => Icerik {
            etiket: "PAKETLER",
            manset: ["Salon boş,", "sahne sizin."],
            alt: "Düğün, nişan ve kurumsal etkinlikler için 400 kişilik salon; ses ve ışık dahil.",
            hikaye_baslik: "Aynı gün tek etkinlik.",
            hikaye: [
                "Mekânı tek etkinliğe açıyoruz: aynı gün iki düğün, iki telaş olmaz. Kurulum sabah başlar, siz gelmeden sahne hazır olur.",
                "Mutfak bizim; menüyü birlikte tadıyoruz. Masa sayısı, ses, ışık ve servis tek planda konuşulur, sonradan sürpriz çıkmaz.",
            ],
            bolum: "Paketler",
            bolum_alt: "Kişi sayısı ve tarihe göre değişir; kesin teklif görüşmede.",
            kalemler: [
                ("Düğün paketi", "400 kişi, menü, ses-ışık, servis"),
                ("Nişan paketi", "200 kişi, kokteyl ve müzik"),
                ("Kurumsal toplantı", "Günlük kiralama, teknik ekip dahil"),
                ("Ses & ışık", "Tek başına kiralama, teknik ekip dahil"),
            ],
            neden: [
                ("Tek plan", "Masa, ses, ışık ve servis tek sözleşmede."),
                ("Otopark", "120 araçlık kapalı otopark, vale hizmeti."),
                ("Kendi mutfağı", "Menü tadımı etkinlikten önce yapılır."),
            ],
            cta: "Tarih sor",
            akis: "Düğün · Nişan · Toplantı · Sahne",
            yorumlar: [
                ("Sözleşmede yazan tutar dışında tek kuruş ödemedik.", "Düğün"),
                ("Kurumsal toplantıda teknik ekip sorunsuz çalıştı.", "Şirket toplantısı"),
            ],
        },
        _ => Icerik {
            etiket: "HİZMETLER",
            manset: ["İşinizi", "anlatan bir sayfa."],
            alt: "Hizmetler, konum ve iletişim tek sayfada; mobilde hızlı açılır.",
            hikaye_baslik: "İşinizi anlatan bir sayfa.",
            hikaye: [
                "Bu sayfa işletmenizin dijital vitrinidir: ne yaptığınızı, nerede olduğunuzu ve size nasıl ulaşılacağını tek ekranda anlatır.",
                "İçerik ve fotoğraflar sizin bilgilerinizle değiştirilir; bu hâliyle bir taslaktır.",
            ],
            bolum: "Hizmetler",
            bolum_alt: "Liste, işletmenizin gerçek hizmetleriyle değiştirilecek.",
            kalemler: [
                ("Hizmet", "Kısa açıklama"),
                ("Hizmet", "Kısa açıklama"),
                ("Hizmet", "Kısa açıklama"),
                ("Hizmet", "Kısa açıklama"),
            ],
            neden: [
                ("Hızlı açılır", "Tek dosya, dış bağımlılık yok."),
                ("Mobil uyumlu", "Telefonda tek elle kullanılır."),
                ("Tek dokunuş", "Arama ve yol tarifi doğrudan."),
            ],
            cta: "İletişime geç",
            akis: "Hizmet · Konum · İletişim",
            yorumlar: [
                ("Bu alan müşteri yorumlarıyla doldurulacak.", "Örnek yorum"),
                ("Bu alan müşteri yorumlarıyla doldurulacak.", "Örnek yorum"),
            ],
        },
    }
}

/// Sayfanın tamamı tek dosyada: harici CSS/JS/font isteği yok.
const CSS: &str = r#"
*,*::before,*::after{box-sizing:border-box}
html{-webkit-text-size-adjust:100%;scroll-behavior:smooth}
body{margin:0;background:var(--bg);color:var(--ink);font-family:var(--f-govde);font-size:16px;line-height:1.62;-webkit-font-smoothing:antialiased;overflow-x:hidden}
img,iframe{max-width:100%;display:block}
a{color:inherit;text-decoration:none}
h1,h2,h3{font-family:var(--f-baslik);font-weight:700;letter-spacing:-.03em;margin:0}
:root{--dim:color-mix(in srgb,var(--ink) 62%,transparent);--cizgi:color-mix(in srgb,var(--ink) 12%,transparent);--kart:color-mix(in srgb,var(--ink) 4%,transparent);--accent-metin:color-mix(in srgb,var(--accent) 76%,var(--ink))}
.kabuk{max-width:1180px;margin:0 auto;padding:0 26px}
.ince{font-size:11.5px;letter-spacing:.22em;text-transform:uppercase;color:var(--dim)}
/* --- tanıtım şeridi --- */
.serit{background:color-mix(in srgb,var(--accent) 14%,var(--bg));color:var(--accent-metin);text-align:center;font-size:11px;letter-spacing:.16em;text-transform:uppercase;padding:9px 18px;border-bottom:1px solid var(--cizgi)}
/* --- üst menü --- */
.ust{position:sticky;top:0;z-index:60;background:color-mix(in srgb,var(--bg) 84%,transparent);backdrop-filter:blur(16px) saturate(1.2);border-bottom:1px solid var(--cizgi);transition:padding .35s ease,box-shadow .35s ease}
.ust-ic{max-width:1180px;margin:0 auto;padding:15px 26px;display:flex;align-items:center;gap:18px}
.ust.kucuk .ust-ic{padding:9px 26px}
.ilerleme{position:fixed;left:0;top:0;height:2px;width:0;background:var(--accent);z-index:80;transition:width .1s linear;box-shadow:0 0 12px var(--accent)}
.marka{display:flex;align-items:center;gap:11px;min-width:0;font-family:var(--f-baslik);font-weight:700;font-size:15.5px;letter-spacing:-.01em}
.monogram{width:36px;height:36px;border-radius:var(--r-1);background:var(--accent);color:var(--accent-karsi);display:grid;place-items:center;font-size:15px;flex:0 0 auto;box-shadow:0 8px 24px -10px var(--accent)}
.marka span{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.menuler{display:flex;align-items:center;gap:26px;margin-left:auto;font-size:13.5px;color:var(--dim)}
.menuler a:hover{color:var(--ink)}
.ust-tel{display:inline-flex;align-items:center;gap:8px;border:1px solid var(--cizgi);border-radius:999px;padding:9px 17px;font-size:13px;font-weight:600;transition:border-color .25s,background .25s}
.ust-tel svg{width:15px;height:15px;flex:0 0 auto}
.ust-tel:hover{border-color:color-mix(in srgb,var(--accent) 60%,transparent);background:color-mix(in srgb,var(--accent) 10%,transparent)}
.mobil-dugme{display:none;margin-left:auto;background:none;border:1px solid var(--cizgi);color:var(--ink);border-radius:var(--r-1);padding:7px 11px;font-size:15px}
.mobil-menu{display:none;border-top:1px solid var(--cizgi);padding:14px 26px 18px;gap:14px;flex-direction:column;font-size:15px;color:var(--dim)}
.mobil-menu.acik{display:flex}
/* --- hero --- */
.hero{position:relative;overflow:hidden;padding:clamp(40px,6vw,84px) 0 clamp(44px,6vw,76px)}
.hero::before,.hero::after{content:"";position:absolute;border-radius:50%;filter:blur(70px);pointer-events:none;z-index:0}
.hero::before{width:min(44vw,440px);height:min(44vw,440px);right:-110px;top:-120px;background:color-mix(in srgb,var(--accent) 34%,transparent);opacity:.5}
.hero::after{width:min(38vw,380px);height:min(38vw,380px);left:-150px;bottom:-150px;background:color-mix(in srgb,var(--accent) 18%,transparent);opacity:.45}
.hero-ic{position:relative;z-index:1;display:grid;grid-template-columns:minmax(0,1.08fr) minmax(0,.92fr);gap:clamp(30px,5vw,72px);align-items:center;min-height:min(68vh,640px)}
.heroi{position:relative;z-index:2}
.rozet-cip{display:inline-flex;align-items:center;gap:9px;border:1px solid color-mix(in srgb,var(--accent) 34%,transparent);border-radius:999px;padding:6px 14px;font-size:11px;letter-spacing:.19em;text-transform:uppercase;color:var(--accent-metin);background:color-mix(in srgb,var(--accent) 9%,transparent)}
.rozet-cip b{width:5px;height:5px;border-radius:50%;background:var(--accent);box-shadow:0 0 12px var(--accent)}
.manset{font-size:clamp(38px,5.7vw,78px);line-height:1.02;letter-spacing:-.042em;margin:22px 0 0}
.manset .manset-satir{display:block;overflow:hidden;padding-bottom:.09em;margin-bottom:-.02em}
.kelime{display:inline-block;overflow:hidden;padding-bottom:.1em;margin-bottom:-.1em;vertical-align:top}
.kelime i{display:block;font-style:normal;transform:translateY(112%);animation:satir .95s cubic-bezier(.19,.78,.18,1) forwards;animation-delay:calc(var(--k,0) * 70ms + 90ms)}
/* Sıcak sektörler: başlık serif. Ölçü ve ağırlık serifin ritmine göre değişir,
   hareket ve düzen aynı kalır. */
body[data-yazi=serif] h1,body[data-yazi=serif] h2,body[data-yazi=serif] h3{font-weight:400;letter-spacing:-.006em}
body[data-yazi=serif] .manset{font-size:clamp(44px,6.5vw,94px);line-height:.95;letter-spacing:-.022em}
body[data-yazi=serif] .bolum h2,body[data-yazi=serif] .bant h2{font-weight:400;letter-spacing:-.012em}
body[data-yazi=serif] .bolum-ust .no{font-style:italic;letter-spacing:.12em}
body[data-yazi=serif] .alinti p{font-family:var(--f-baslik);font-size:21px;line-height:1.42;letter-spacing:0}
/* Küçük puntoda serif okunmaz ve yapay kalınlaşır: arayüz yazıları gövde
   fontunda kalır, serif yalnız büyük başlık ve alıntıda kullanılır. */
body[data-yazi=serif] .marka,body[data-yazi=serif] .alt-marka,body[data-yazi=serif] .ist b,body[data-yazi=serif] .rozet b,body[data-yazi=serif] .satir h3,body[data-yazi=serif] .kart h3,body[data-yazi=serif] .ozet b,body[data-yazi=serif] .alinti footer b,body[data-yazi=serif] .akis-ic span{font-family:var(--f-govde);font-weight:600;letter-spacing:-.005em}
.hero-alt{margin:22px 0 0;max-width:52ch;color:var(--dim);font-size:clamp(14.5px,1.35vw,17px);line-height:1.68}
.hero-butonlar{display:flex;flex-wrap:wrap;gap:11px;margin-top:30px}
.buton{display:inline-flex;align-items:center;gap:9px;border-radius:999px;padding:14px 24px;font-size:14px;font-weight:650;letter-spacing:.01em;transition:transform .25s cubic-bezier(.2,.8,.2,1),box-shadow .25s,background .25s,border-color .25s}
.buton svg{width:15px;height:15px}
.buton-dolu{background:var(--accent);color:var(--accent-karsi);box-shadow:0 16px 40px -18px var(--accent)}
.buton-dolu:hover{transform:translateY(-2px);box-shadow:0 22px 48px -18px var(--accent)}
.buton-cizgi{border:1px solid var(--cizgi);color:var(--ink)}
.buton-cizgi:hover{border-color:color-mix(in srgb,var(--accent) 55%,transparent);background:color-mix(in srgb,var(--accent) 8%,transparent)}
.durum{margin-top:26px;display:inline-flex;align-items:center;gap:10px;font-size:13.5px;color:var(--dim)}
.durum b{font-weight:650}
.nokta-canli{width:7px;height:7px;border-radius:50%;background:#4ade80;box-shadow:0 0 0 0 rgba(74,222,128,.6);animation:nabiz 2.4s infinite}
.nokta-canli.kapali{background:color-mix(in srgb,var(--ink) 38%,transparent);animation:none}
.durum-metin{color:color-mix(in srgb,var(--ink) 62%,transparent)}
.durum-metin.acik{color:#6ee7a8}
.durum-metin.yakin{color:#fbbf24}
.tema-acik .durum-metin.acik{color:#1c7a4a}
.tema-acik .durum-metin.yakin{color:#9a5b06}
.tema-acik .nokta-canli.kapali{background:color-mix(in srgb,var(--ink) 30%,transparent)}
.nokta-canli.yakin{background:#fbbf24}
/* --- sahne (fotoğrafsız kompozisyon) --- */
.sahne{position:relative;aspect-ratio:4/5;border-radius:var(--r-4);overflow:hidden;border:1px solid color-mix(in srgb,var(--accent) 22%,transparent);background:radial-gradient(120% 95% at 78% 6%,color-mix(in srgb,var(--accent) 34%,transparent),transparent 62%),radial-gradient(95% 75% at 8% 96%,color-mix(in srgb,var(--accent) 16%,transparent),transparent 66%),linear-gradient(158deg,color-mix(in srgb,var(--ink) 7%,var(--bg)),var(--bg))}
.sahne{animation:sahne-ac .95s cubic-bezier(.19,.78,.18,1) both}
@keyframes sahne-ac{from{clip-path:inset(16% 0 0 0);opacity:.55}to{clip-path:inset(0 0 0 0);opacity:1}}
.halkalar{position:absolute;inset:-38%;background:repeating-radial-gradient(circle at 58% 44%,transparent 0 22px,color-mix(in srgb,var(--ink) 7%,transparent) 22px 23px);animation:don 76s linear infinite}
.sahne-foto{position:absolute;inset:0;width:100%;height:100%;object-fit:cover;object-position:50% 42%;filter:grayscale(.18) contrast(1.06) brightness(.9) saturate(.96);transform:scale(1.03);transition:transform 1.2s cubic-bezier(.2,.8,.2,1)}
.sahne:hover .sahne-foto{transform:scale(1.07)}
.sahne-perde{position:absolute;inset:0;background:linear-gradient(180deg,color-mix(in srgb,var(--accent) 34%,transparent),transparent 46%,color-mix(in srgb,var(--bg) 55%,transparent));mix-blend-mode:soft-light}
.sahne-koyu{position:absolute;inset:0;background:radial-gradient(115% 85% at 52% 42%,transparent 28%,color-mix(in srgb,var(--bg) 74%,transparent) 100%)}
.monogram-buyuk{position:absolute;left:50%;top:47%;transform:translate(-50%,-50%);font-family:var(--f-baslik);font-weight:700;font-size:min(26vw,260px);line-height:.8;color:transparent;-webkit-text-stroke:1px color-mix(in srgb,var(--accent) 50%,transparent);opacity:.55;user-select:none}
.sahne-alt{position:absolute;left:24px;right:24px;bottom:22px;display:flex;align-items:flex-end;justify-content:space-between;gap:14px;font-size:10.5px;letter-spacing:.24em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 45%,transparent)}
/* Fotoğraf üzerindeki yazı her temada açık renk kalır. */
.sahne.foto .sahne-alt{color:rgba(255,255,255,.68);text-shadow:0 1px 10px rgba(0,0,0,.5)}
.sahne.foto .sahne-koyu{background:linear-gradient(180deg,transparent 42%,rgba(0,0,0,.5)),radial-gradient(115% 85% at 52% 42%,transparent 30%,color-mix(in srgb,var(--bg) 62%,transparent) 100%)}
.rozet{position:absolute;border-radius:var(--r-2);padding:12px 15px;background:color-mix(in srgb,var(--bg) 78%,transparent);backdrop-filter:blur(12px);border:1px solid var(--cizgi);box-shadow:0 18px 40px -24px color-mix(in srgb,var(--bg) 80%,#000)}
.rozet small{display:block;font-size:9.5px;letter-spacing:.2em;text-transform:uppercase;color:var(--dim);margin-bottom:3px}
.rozet b{font-family:var(--f-baslik);font-size:15px;letter-spacing:-.01em;font-weight:650}
.rozet-puan{display:flex;align-items:baseline;gap:7px}
.rozet-puan small{display:inline;margin:0;font-size:10.5px;letter-spacing:.08em;text-transform:none}
.rozet-a{left:20px;top:20px;animation:yuz 9s ease-in-out infinite;min-width:132px}
.rozet-b{right:20px;bottom:70px;animation:yuz 11s ease-in-out infinite reverse;min-width:158px}
.yildizlar{display:flex;gap:2px;margin-bottom:5px}
.yildizlar svg{width:11px;height:11px;color:var(--accent)}
/* --- istatistik şeridi --- */
.istatistik{border-top:1px solid var(--cizgi);border-bottom:1px solid var(--cizgi);margin-top:clamp(26px,4vw,44px)}
.istatistik-ic{display:grid;grid-template-columns:repeat(4,1fr);max-width:1180px;margin:0 auto;padding:0 26px}
.ist{background:var(--bg);padding:20px 22px;border-left:1px solid var(--cizgi)}
.ist:first-child{padding-left:0;border-left:0}
.ist small{display:block;font-size:10px;letter-spacing:.2em;text-transform:uppercase;color:var(--dim);margin-bottom:6px}
.ist b{font-family:var(--f-baslik);font-size:15px;font-weight:650;letter-spacing:-.01em}
/* --- akan şerit --- */
.akis{overflow:hidden;border-bottom:1px solid var(--cizgi);background:color-mix(in srgb,var(--accent) 6%,transparent);padding:15px 0}
.akis-ic{display:flex;width:max-content;animation:kay 38s linear infinite}
.akis-ic span{font-family:var(--f-baslik);font-size:13px;letter-spacing:.24em;text-transform:uppercase;color:var(--accent-metin);padding:0 22px;white-space:nowrap;display:inline-flex;align-items:center;gap:22px}
.akis-ic span::after{content:"";width:4px;height:4px;border-radius:50%;background:color-mix(in srgb,var(--accent) 60%,transparent)}
/* --- bölümler --- */
.bolum{padding:clamp(40px,4.8vw,66px) 0}
.bolum-ust{position:relative;display:flex;align-items:baseline;gap:14px;padding-top:16px;border-top:0;margin-bottom:clamp(26px,4vw,46px)}
.bolum-ust::before{content:"";position:absolute;left:0;right:0;top:0;height:1px;background:var(--cizgi);transform:scaleX(0);transform-origin:left center;transition:transform 1.2s cubic-bezier(.19,.78,.18,1)}
.bolum-ust.in::before{transform:scaleX(1)}
.bolum-ust .no{font-family:var(--f-baslik);font-size:11.5px;letter-spacing:.2em;color:var(--accent)}
.bolum h2{font-size:clamp(26px,3.8vw,44px);line-height:1.06;max-width:22ch}
.hikaye{display:grid;grid-template-columns:minmax(0,.9fr) minmax(0,1.1fr);gap:clamp(24px,5vw,64px);align-items:start}
.hikaye.fotolu{grid-template-columns:minmax(0,1.05fr) minmax(0,.95fr);align-items:center}
.hikaye-metin{display:grid;gap:18px}
.hikaye-metin h2{margin:0}
.hikaye-gorsel{margin:0;border-radius:var(--r-3);overflow:hidden;border:1px solid var(--cizgi);background:var(--kart);aspect-ratio:4/5;max-height:520px}
.hikaye-gorsel img{width:100%;height:100%;object-fit:cover;object-position:50% 45%;transform:scale(1.02);transition:transform 1.4s cubic-bezier(.2,.8,.2,1)}
.hikaye-gorsel:hover img{transform:scale(1.06)}
.hikaye-buyuk{font-size:clamp(18px,2.1vw,24px);line-height:1.52;color:var(--ink);letter-spacing:-.015em;margin:0}
.hikaye p{color:var(--dim);font-size:15px;margin:18px 0 0;max-width:60ch}
.ozet-kutu{display:grid;grid-template-columns:repeat(3,1fr);gap:1px;background:var(--cizgi);border:1px solid var(--cizgi);border-radius:var(--r-2);overflow:hidden;margin-top:clamp(28px,4vw,44px)}
.ozet{background:linear-gradient(180deg,rgba(255,255,255,.03),transparent);padding:20px 18px}
.ozet b{display:block;font-family:var(--f-baslik);font-size:13px;letter-spacing:.01em;margin-bottom:5px}
.ozet span{font-size:13px;color:var(--dim);line-height:1.5;display:block}
/* --- menü / hizmet listesi --- */
.liste{list-style:none;margin:0;padding:0;border-top:1px solid var(--cizgi)}
.satir{padding:20px 0;border-bottom:1px solid var(--cizgi);transition:padding-left .3s ease,background .3s ease}
.satir:hover{padding-left:14px;background:linear-gradient(90deg,color-mix(in srgb,var(--accent) 7%,transparent),transparent 70%)}
.satir-ust{display:flex;align-items:baseline}
.satir h3{font-size:clamp(16px,1.6vw,19px);font-weight:650;letter-spacing:-.02em}
.satir p{margin:5px 0 0;color:var(--dim);font-size:14px;max-width:64ch}
.liste-alt{margin-top:16px;font-size:12px;color:color-mix(in srgb,var(--dim) 80%,transparent)}
/* --- galeri (işletmenin kendi fotoğrafları) --- */
.galeri-ust{display:flex;align-items:baseline;justify-content:space-between;gap:12px;margin-top:38px;padding-top:18px;border-top:1px solid var(--cizgi)}
.galeri-not{font-size:11px;color:color-mix(in srgb,var(--dim) 82%,transparent)}
.galeri{display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:12px;margin-top:16px}
.galeri-kare{margin:0;position:relative;overflow:hidden;border-radius:var(--r-2);border:1px solid var(--cizgi);aspect-ratio:4/3;background:var(--kart)}
.galeri-kare img{width:100%;height:100%;object-fit:cover;transform:scale(1.04);transition:transform 1.1s cubic-bezier(.2,.8,.2,1)}
.galeri-kare:hover img{transform:scale(1.12)}
/* --- neden biz --- */
.uc{display:grid;grid-template-columns:repeat(3,1fr);gap:18px;margin-top:8px}
.kart{border:1px solid var(--cizgi);border-radius:var(--r-3);padding:26px 22px;background:linear-gradient(180deg,color-mix(in srgb,var(--ink) 4%,transparent),transparent);transition:transform .35s cubic-bezier(.2,.8,.2,1),border-color .35s;position:relative;overflow:hidden}
.kart:hover{transform:translateY(-4px);border-color:color-mix(in srgb,var(--accent) 45%,transparent)}
.kart .sira{font-family:var(--f-baslik);font-size:11px;letter-spacing:.2em;color:var(--accent);display:block;margin-bottom:16px}
.kart h3{font-size:17px;font-weight:650;margin-bottom:9px}
.kart p{margin:0;color:var(--dim);font-size:14px;line-height:1.6}
/* --- yorumlar --- */
.yorumlar{display:grid;grid-template-columns:repeat(2,1fr);gap:18px}
.alinti{margin:0;border:1px solid var(--cizgi);border-radius:var(--r-3);padding:26px 24px;background:var(--kart);position:relative}
.alinti::before{content:"”";position:absolute;right:20px;top:4px;font-family:var(--f-baslik);font-size:70px;line-height:1;color:color-mix(in srgb,var(--accent) 20%,transparent)}
.alinti p{margin:0 0 16px;font-size:15.5px;line-height:1.6;color:var(--ink);position:relative;z-index:1}
.alinti footer{display:flex;align-items:center;gap:10px;font-size:12.5px;color:var(--dim)}
.alinti footer b{width:26px;height:26px;border-radius:var(--r-1);display:grid;place-items:center;background:color-mix(in srgb,var(--accent) 14%,transparent);color:var(--accent-metin);font-family:var(--f-baslik);font-size:11px;letter-spacing:.04em}
.ornek-not{font-size:11px;letter-spacing:.12em;text-transform:uppercase;color:color-mix(in srgb,var(--dim) 70%,transparent);margin-top:14px}
/* --- saatler & iletişim --- */
.iletisim{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:clamp(22px,4vw,44px);align-items:start}
.saatler{border:1px solid var(--cizgi);border-radius:var(--r-3);overflow:hidden}
.saat-satir{display:flex;justify-content:space-between;gap:16px;padding:12px 18px;font-size:14px;border-bottom:1px solid var(--cizgi);color:var(--dim)}
.saat-satir:last-child{border-bottom:0}
.saat-satir.bugun{background:color-mix(in srgb,var(--accent) 9%,transparent);color:var(--ink);font-weight:600}
.saat-satir.bugun span:first-child::before{content:"";display:inline-block;width:6px;height:6px;border-radius:50%;background:var(--accent);margin-right:9px;vertical-align:middle}
.saat-satir b{font-weight:inherit}
.ilet-kartlar{display:grid;gap:12px;margin-top:16px}
.ilet-kart{border:1px solid var(--cizgi);border-radius:var(--r-2);padding:16px 18px;display:flex;gap:13px;align-items:flex-start;transition:border-color .3s,background .3s}
.ilet-kart:hover{border-color:color-mix(in srgb,var(--accent) 45%,transparent);background:color-mix(in srgb,var(--accent) 6%,transparent)}
.ilet-kart svg{width:17px;height:17px;flex:0 0 auto;margin-top:3px;color:var(--accent)}
.ilet-kart small{display:block;font-size:10px;letter-spacing:.18em;text-transform:uppercase;color:var(--dim);margin-bottom:3px}
.ilet-kart b{font-weight:600;font-size:14px;line-height:1.5}
.harita{border:1px solid var(--cizgi);border-radius:var(--r-3);overflow:hidden;background:var(--bg);display:flex;flex-direction:column}
.harita iframe{width:100%;height:400px;border:0;filter:grayscale(1) invert(.92) contrast(.88) brightness(.95) hue-rotate(180deg)}
.harita-kart{margin-top:14px;border:1px solid color-mix(in srgb,var(--accent) 26%,transparent);background:linear-gradient(160deg,color-mix(in srgb,var(--accent) 12%,transparent),transparent);border-radius:var(--r-3);padding:22px 22px}
.harita-alt{display:flex;justify-content:space-between;align-items:center;padding:12px 16px;font-size:12.5px;color:var(--dim);border-top:1px solid var(--cizgi)}
.harita-alt a{color:var(--accent);font-weight:600}
.suresi-doldu{position:fixed;inset:0;z-index:999;background:var(--bg);color:var(--ink);display:grid;place-items:center;text-align:center;padding:24px;font-size:17px;line-height:1.6}
.suresi-doldu[hidden]{display:none}
/* --- kapanış bandı --- */
.bant{background:var(--accent);color:var(--accent-karsi);padding:clamp(46px,6vw,78px) 0;position:relative;overflow:hidden}
.bant::after{content:"";position:absolute;inset:0;background:repeating-linear-gradient(115deg,rgba(0,0,0,.045) 0 2px,transparent 2px 26px);pointer-events:none}
.bant-ic{position:relative;z-index:1;display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:26px}
.bant h2{font-size:clamp(26px,4vw,46px);line-height:1.05;max-width:20ch;color:var(--accent-karsi)}
.bant p{margin:10px 0 0;font-size:14.5px;color:color-mix(in srgb,var(--accent-karsi) 76%,transparent);max-width:44ch}
.bant .buton{background:var(--accent-karsi);color:var(--accent);box-shadow:none}
.bant .buton:hover{transform:translateY(-2px)}
/* --- altbilgi --- */
.altbilgi{border-top:1px solid var(--cizgi);padding:44px 0 40px;font-size:12.5px;color:var(--dim)}
.alt-ic{display:flex;flex-wrap:wrap;gap:22px;justify-content:space-between;align-items:flex-start}
.alt-marka{display:flex;align-items:center;gap:11px;font-family:var(--f-baslik);font-weight:650;color:var(--ink);font-size:14px}
.alt-baglar{display:flex;flex-wrap:wrap;gap:18px}
.alt-baglar a:hover{color:var(--ink)}
.alt-not{max-width:52ch;line-height:1.6;margin:14px 0 0}
.alt-not a{color:var(--accent)}
/* --- satıcı bandı: taslak siteyi özel yapma teklifi --- */
.satici{border-top:1px solid var(--cizgi);background:color-mix(in srgb,var(--accent) 7%,transparent)}
.satici-ic{display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:22px;padding:clamp(26px,3.6vw,40px) 0}
.satici h2{font-size:clamp(20px,2.4vw,27px);line-height:1.15;margin:8px 0 0;max-width:24ch}
.satici p{margin:8px 0 0;color:var(--dim);font-size:14px;max-width:46ch}
.satici .buton{flex:0 0 auto}
/* --- sabit mobil CTA --- */
.mobil-cta{display:none;position:fixed;left:12px;right:12px;bottom:12px;z-index:70;border-radius:var(--r-2);background:var(--accent);color:var(--accent-karsi);text-align:center;font-weight:700;padding:15px;box-shadow:0 16px 40px -14px rgba(0,0,0,.8)}
/* --- ışık & doku --- */
#fener{position:fixed;inset:0;z-index:3;pointer-events:none;opacity:0;transition:opacity .4s ease;background:radial-gradient(340px circle at var(--fx,50%) var(--fy,50%),color-mix(in srgb,var(--accent) 20%,transparent),transparent 68%);mix-blend-mode:screen}
@media (hover:hover) and (pointer:fine){#fener.acik{opacity:1}}
.gren{position:fixed;inset:0;z-index:2;pointer-events:none;opacity:.032;background-image:url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='160' height='160'><filter id='g'><feTurbulence type='fractalNoise' baseFrequency='.85' numOctaves='3'/></filter><rect width='160' height='160' filter='url(%23g)'/></svg>")}
/* --- hareket --- */
[data-rise]{opacity:0;transform:translateY(24px);transition:opacity .8s cubic-bezier(.19,.78,.18,1),transform .8s cubic-bezier(.19,.78,.18,1);transition-delay:calc(var(--g,0) * 1ms)}
[data-rise].in{opacity:1;transform:none}
@keyframes satir{to{transform:translateY(0)}}
@keyframes kay{to{transform:translateX(-50%)}}
@keyframes don{to{transform:rotate(360deg)}}
@keyframes yuz{0%,100%{transform:translateY(0)}50%{transform:translateY(-13px)}}
@keyframes nabiz{0%{box-shadow:0 0 0 0 rgba(74,222,128,.55)}70%{box-shadow:0 0 0 9px rgba(74,222,128,0)}100%{box-shadow:0 0 0 0 rgba(74,222,128,0)}}
/* --- duyarlı --- */
@media (max-width:1000px){
  .hero-ic{grid-template-columns:1fr;min-height:0;gap:34px}
  .sahne{aspect-ratio:16/10;max-height:340px;max-width:100%}
  .monogram-buyuk{font-size:min(34vw,190px)}
  .sahne-alt{display:none}
  .rozet-a{left:12px;top:12px;min-width:0;padding:10px 13px}
  .rozet-b{right:12px;bottom:12px;min-width:0;padding:10px 13px}
  .serit{font-size:10px;letter-spacing:.12em}
  .hikaye{grid-template-columns:1fr}
  .uc{grid-template-columns:1fr}
  .yorumlar{grid-template-columns:1fr}
  .iletisim{grid-template-columns:1fr}
  .istatistik-ic{grid-template-columns:repeat(2,1fr);padding:0 26px}
  .ist{padding:16px 14px}
  .ist:nth-child(odd){padding-left:0;border-left:0}
  .ist:nth-child(n+3){border-top:1px solid var(--cizgi)}
  .menuler,.ust-tel{display:none}
  .mobil-dugme{display:block}
  .mobil-cta{display:block}
  body{padding-bottom:74px}
  .harita iframe{height:320px}
}
@media (max-width:1000px){
  .hikaye-gorsel{aspect-ratio:16/10;max-height:320px;order:-1}
}
@media (max-width:560px){
  .kabuk,.ust-ic{padding-left:18px;padding-right:18px}
  .satici-ic{gap:16px}
  .ozet-kutu{grid-template-columns:1fr}
  .buton{padding:13px 20px}
  .galeri{grid-template-columns:1fr 1fr}
}
.tema-acik .harita iframe{filter:grayscale(.25) contrast(1.02)}
.tema-acik #fener{mix-blend-mode:multiply;opacity:0}
.tema-acik .gren{opacity:.022}
@media (prefers-reduced-motion:reduce){
  *,*::before,*::after{animation-duration:.001ms!important;animation-iteration-count:1!important;transition-duration:.001ms!important}
  [data-rise]{opacity:1;transform:none}
  .manset .kelime i{transform:none}
  .bolum-ust::before{transform:scaleX(1)}
  .sahne{animation:none}
  #fener,.gren{display:none}
}
"#;

/// Acil çağrı sektörleri (çekici, yol yardım, çilingir, acil servis): sayfada
/// canlı konum gönderme, 7/24 rozetleri ve yol yardım asistanı çizilir.
pub fn acil_sektor(sektor: &str) -> bool {
    let sade: String = sektor
        .chars()
        .map(|harf| match harf {
            'Ç' | 'ç' => 'c',
            'Ğ' | 'ğ' => 'g',
            'İ' | 'I' | 'ı' | 'i' => 'i',
            'Ö' | 'ö' => 'o',
            'Ş' | 'ş' => 's',
            'Ü' | 'ü' => 'u',
            diger => diger.to_ascii_lowercase(),
        })
        .collect();
    ["cekici", "kurtarma", "yol yardim", "cilingir", "acil"]
        .iter()
        .any(|anahtar| sade.contains(anahtar))
}

/// Rengin algılanan parlaklığı; açık temada gölge/karartma yönü değişir.
pub fn parlak_mi(hex: &str) -> bool {
    let temiz = hex.trim_start_matches('#');
    if temiz.len() != 6 {
        return false;
    }
    let kanal = |bas: usize| u8::from_str_radix(&temiz[bas..bas + 2], 16).unwrap_or(0) as f32;
    let (r, g, b) = (kanal(0), kanal(2), kanal(4));
    0.2126 * r + 0.7152 * g + 0.0722 * b > 140.0
}

/// Panelden yazılan metin: boşsa şablon, değilse yazılan (kaçırılmış).
/// Tek satırlık alanlar 120, paragraflar daha uzun tutulabildiği için
/// kırpma sınırı çağıran tarafta (`main.rs`) uygulanır.
/// Manşet satırını kelime kelime maskeye alır: her kelime kendi gecikmesiyle
/// aşağıdan yukarı kayar. Gecikme sırası satırlar arasında sürer, böylece
/// ikinci satır birincinin bitişini bekler.
fn manset_satiri(metin: &str, baslangic: usize) -> (String, usize) {
    let mut sayac = baslangic;
    let kelimeler: Vec<String> = metin
        .split_whitespace()
        .map(|kelime| {
            let gecikme = sayac;
            sayac += 1;
            format!(
                "<span class=\"kelime\" style=\"--k:{gecikme}\"><i>{}</i></span>",
                kacis(kelime)
            )
        })
        .collect();
    (
        format!(
            "<span class=\"manset-satir\">{}</span>",
            kelimeler.join(" ")
        ),
        sayac,
    )
}

/// Bir başlığı kelime kelime maskeye alır: her kelime kendi gecikmesiyle
/// aşağıdan süzülür ve bulanıklıktan netleşir. Fasıl `.etkin` olunca oynar.
fn kelimeler(metin: &str) -> String {
    metin
        .split_whitespace()
        .enumerate()
        .map(|(sira, kelime)| {
            format!(
                "<span class=\"klm\" style=\"--i:{sira}\"><i>{}</i></span>",
                kacis(kelime)
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// URL metni için yüzde kodlaması: boşluk ve Türkçe harfler wa.me bağlantısında
/// bozulmasın. Yalnız harf, rakam ve `-._~` olduğu gibi kalır.
fn url_kacis(metin: &str) -> String {
    let mut cikti = String::with_capacity(metin.len() * 3);
    for bayt in metin.as_bytes() {
        let karakter = *bayt as char;
        if karakter.is_ascii_alphanumeric() || matches!(karakter, '-' | '.' | '_' | '~') {
            cikti.push(karakter);
        } else {
            cikti.push_str(&format!("%{bayt:02X}"));
        }
    }
    cikti
}

fn ozel(metin: &str, varsayilan: &str) -> String {
    let temiz = metin.trim();
    kacis(if temiz.is_empty() { varsayilan } else { temiz })
}

/// HTML özel karakterlerini kaçırır: işletme adı/not kullanıcı verisidir.
fn kacis(metin: &str) -> String {
    let mut sonuc = String::with_capacity(metin.len());
    for karakter in metin.chars() {
        match karakter {
            '&' => sonuc.push_str("&amp;"),
            '<' => sonuc.push_str("&lt;"),
            '>' => sonuc.push_str("&gt;"),
            '"' => sonuc.push_str("&quot;"),
            '\'' => sonuc.push_str("&#39;"),
            _ => sonuc.push(karakter),
        }
    }
    sonuc
}

/// Adresin son parçası genelde ilçe/il olur; istatistik şeridinde kullanılır.
/// Posta kodu gibi yalnız rakamlardan oluşan parçalar atılır.
fn ilce_metni(adres: &str) -> String {
    let son = adres
        .split(',')
        .map(str::trim)
        .rfind(|parca| !parca.is_empty())
        .unwrap_or("");
    let temiz: Vec<&str> = son
        .split_whitespace()
        .filter(|parca| !parca.chars().all(|harf| harf.is_ascii_digit()))
        .collect();
    let ad = temiz.join(" ");
    if ad.is_empty() {
        "Konum".to_string()
    } else {
        kacis(&ad)
    }
}

/// Konum kartındaki il etiketi: "Paşa Alanı, 10020 Balıkesir" → "Balıkesir".
fn il_metni(adres: &str) -> String {
    let son = adres
        .rsplit([',', '/'])
        .map(str::trim)
        .find(|parca| !parca.is_empty())
        .unwrap_or("");
    let temiz: Vec<&str> = son
        .split_whitespace()
        .filter(|parca| !parca.chars().all(|harf| harf.is_ascii_digit()))
        .collect();
    let ad = temiz.join(" ");
    if ad.is_empty() {
        "Türkiye".to_string()
    } else {
        format!("{} · Türkiye", kacis(&ad))
    }
}

/// Lucide (ISC lisansı) ikon şablonları: 24×24, `currentColor`, ince çizgi.
/// Gömülüdür; harici istek, CDN veya ikon fontu yoktur.
const LUCIDE_KALKAN: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/></svg>"#;
const LUCIDE_KIVILCIM: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z"/><path d="M20 2v4"/><path d="M22 4h-4"/><circle cx="4" cy="20" r="2"/></svg>"#;
const LUCIDE_SAAT: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>"#;
const LUCIDE_BOT: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 8V4H8"/><rect width="16" height="12" x="4" y="8" rx="2"/><path d="M2 14h2"/><path d="M20 14h2"/><path d="M15 13v2"/><path d="M9 13v2"/></svg>"#;
const LUCIDE_ROZET: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.85 8.62a4 4 0 0 1 4.78-4.77 4 4 0 0 1 6.74 0 4 4 0 0 1 4.78 4.78 4 4 0 0 1 0 6.74 4 4 0 0 1-4.77 4.78 4 4 0 0 1-6.75 0 4 4 0 0 1-4.78-4.77 4 4 0 0 1 0-6.76Z"/><path d="m16 9-5.5 5.5L8 12"/></svg>"#;
const LUCIDE_PUSULA: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="m16.24 7.76-1.804 5.411a2 2 0 0 1-1.265 1.265L7.76 16.24l1.804-5.411a2 2 0 0 1 1.265-1.265z"/></svg>"#;
const LUCIDE_ANAHTAR: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.106-3.105c.32-.322.863-.22.983.218a6 6 0 0 1-8.259 7.057l-7.91 7.91a1 1 0 0 1-2.999-3l7.91-7.91a6 6 0 0 1 7.057-8.259c.438.12.54.662.219.984z"/></svg>"#;
const LUCIDE_KAMYON: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14 18V6a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2v11a1 1 0 0 0 1 1h2"/><path d="M15 18H9"/><path d="M19 18h2a1 1 0 0 0 1-1v-3.65a1 1 0 0 0-.22-.624l-3.48-4.35A1 1 0 0 0 17.52 8H14"/><circle cx="17" cy="18" r="2"/><circle cx="7" cy="18" r="2"/></svg>"#;
const LUCIDE_EKIP: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><path d="M16 3.128a4 4 0 0 1 0 7.744"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><circle cx="9" cy="7" r="4"/></svg>"#;
const LUCIDE_MAKBUZ: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 17V7"/><path d="M16 8h-6a2 2 0 0 0 0 4h4a2 2 0 0 1 0 4H8"/><path d="M4 3a1 1 0 0 1 1-1 1.3 1.3 0 0 1 .7.2l.933.6a1.3 1.3 0 0 0 1.4 0l.934-.6a1.3 1.3 0 0 1 1.4 0l.933.6a1.3 1.3 0 0 0 1.4 0l.933-.6a1.3 1.3 0 0 1 1.4 0l.934.6a1.3 1.3 0 0 0 1.4 0l.933-.6A1.3 1.3 0 0 1 19 2a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1 1.3 1.3 0 0 1-.7-.2l-.933-.6a1.3 1.3 0 0 0-1.4 0l-.934.6a1.3 1.3 0 0 1-1.4 0l-.933-.6a1.3 1.3 0 0 0-1.4 0l-.933.6a1.3 1.3 0 0 1-1.4 0l-.934-.6a1.3 1.3 0 0 0-1.4 0l-.933.6a1.3 1.3 0 0 1-.7.2 1 1 0 0 1-1-1z"/></svg>"#;
const LUCIDE_MESAJ: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.992 16.342a2 2 0 0 1 .094 1.167l-1.065 3.29a1 1 0 0 0 1.236 1.168l3.413-.998a2 2 0 0 1 1.099.092 10 10 0 1 0-4.777-4.719"/></svg>"#;
const LUCIDE_TIK: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>"#;
const LUCIDE_ODUL: &str = r#"<svg class="aj-ikon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m15.477 12.89 1.515 8.526a.5.5 0 0 1-.81.47l-3.58-2.687a1 1 0 0 0-1.197 0l-3.586 2.686a.5.5 0 0 1-.81-.469l1.514-8.526"/><circle cx="12" cy="8" r="6"/></svg>"#;

/// Ad → gömülü Lucide ikonu. Bilinmeyen ad nötr bir ışıltıya düşer.
fn ikon(ad: &str) -> &'static str {
    match ad {
        "kalkan" => LUCIDE_KALKAN,
        "saat" => LUCIDE_SAAT,
        "bot" => LUCIDE_BOT,
        "rozet" => LUCIDE_ROZET,
        "pusula" => LUCIDE_PUSULA,
        "anahtar" => LUCIDE_ANAHTAR,
        "kamyon" => LUCIDE_KAMYON,
        "ekip" => LUCIDE_EKIP,
        "makbuz" => LUCIDE_MAKBUZ,
        "mesaj" => LUCIDE_MESAJ,
        "tik" => LUCIDE_TIK,
        "odul" => LUCIDE_ODUL,
        _ => LUCIDE_KIVILCIM,
    }
}

/// Hizmet kartlarının ikon sırası sektör karakterine göre değişir.
fn hizmet_ikonlari(grup: &str) -> [&'static str; 4] {
    match grup {
        "acil" => ["kamyon", "kalkan", "kivilcim", "saat"],
        "mutfak" | "konaklama" | "guzellik" | "etkinlik" => ["kivilcim", "odul", "ekip", "saat"],
        "saglik" | "kurumsal" | "egitim" => ["kalkan", "ekip", "saat", "makbuz"],
        _ => ["pusula", "anahtar", "ekip", "kalkan"],
    }
}

const YILDIZ: &str = r#"<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 2.6l2.9 5.9 6.5.95-4.7 4.6 1.1 6.5L12 17.5l-5.8 3.05 1.1-6.5-4.7-4.6 6.5-.95z"/></svg>"#;
const TELEFON_IKON: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M22 16.9v3a2 2 0 0 1-2.2 2 19.8 19.8 0 0 1-8.6-3.1 19.5 19.5 0 0 1-6-6A19.8 19.8 0 0 1 2.1 4.2 2 2 0 0 1 4.1 2h3a2 2 0 0 1 2 1.7c.1 1 .4 1.9.7 2.8a2 2 0 0 1-.5 2.1L8.1 9.9a16 16 0 0 0 6 6l1.3-1.3a2 2 0 0 1 2.1-.4c.9.3 1.8.6 2.8.7a2 2 0 0 1 1.7 2z"/></svg>"#;
const KONUM_IKON: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M20 10c0 6-8 12-8 12s-8-6-8-12a8 8 0 0 1 16 0z"/><circle cx="12" cy="10" r="3"/></svg>"#;
fn harita_html(g: &Girdi, ad: &str) -> String {
    let Some((enlem, boylam)) = g.konum else {
        return String::new();
    };
    // Demo sayfası ağ isteği yapmaz: harita yerine koordinat kartı ve
    // Google Maps bağlantısı verilir (gömme harita kotadan istek harcamaz).
    let ac = format!("https://www.google.com/maps?q={enlem:.5},{boylam:.5}");
    let il = il_metni(&g.adres);
    format!(
        r##"<div class="harita">
<div class="harita-yerel" style="display:flex;flex-direction:column;gap:6px;align-items:center;justify-content:center;min-height:230px;border:1px solid currentColor;border-radius:16px;padding:22px;text-align:center">
<span style="font-size:11px;letter-spacing:.18em;text-transform:uppercase;opacity:.62">Konum</span>
<b style="font-size:17px">{ad}</b>
<small style="font-size:12px;opacity:.72">{enlem:.5}, {boylam:.5}</small>
</div>
<div class="harita-alt"><span>{il}</span><a href="{ac}" target="_blank" rel="noopener">Google Maps'te aç ↗</a></div>
</div>"##
    )
}

fn saatler_html(g: &Girdi) -> String {
    if g.saat_satirlari.is_empty() {
        return format!(
            r##"<div class="saatler"><div class="saat-satir"><span>Çalışma saatleri</span><b>{}</b></div></div>"##,
            if g.saat_metni.is_empty() {
                "Telefonla öğrenin".to_string()
            } else {
                kacis(&g.saat_metni)
            }
        );
    }
    let mut satirlar = String::new();
    for (sira, (gun, saat)) in g.saat_satirlari.iter().enumerate() {
        satirlar.push_str(&format!(
            r##"<div class="saat-satir" data-gun="{sira}"><span>{}</span><b>{}</b></div>"##,
            kacis(gun),
            kacis(saat)
        ));
    }
    format!(r##"<div class="saatler">{satirlar}</div>"##)
}

fn istatistik_html(g: &Girdi, bolum: &str) -> String {
    let mut kalemler = vec![
        (
            "Bugün".to_string(),
            r#"<b id="ist-bugun">—</b>"#.to_string(),
        ),
        (
            "Bölge".to_string(),
            format!("<b>{}</b>", ilce_metni(&g.adres)),
        ),
    ];
    match (&g.puan, &g.yorum) {
        (Some(puan), Some(yorum)) => {
            kalemler.push(("Yorumlar".to_string(), format!("<b>{puan} ★</b> · {yorum}")))
        }
        (Some(puan), None) => kalemler.push(("Yorumlar".to_string(), format!("<b>{puan} ★</b>"))),
        _ => kalemler.push(("Bölüm".to_string(), format!("<b>{bolum}</b>"))),
    }
    kalemler.push((
        "İletişim".to_string(),
        format!(
            "<b>{}</b>",
            if g.telefon.is_some() {
                kacis(&g.telefon_goster)
            } else {
                "Telefonla".to_string()
            }
        ),
    ));
    let mut govde = String::new();
    for (etiket, deger) in kalemler {
        govde.push_str(&format!(
            r##"<div class="ist"><small>{etiket}</small>{deger}</div>"##
        ));
    }
    format!(r##"<div class="istatistik"><div class="istatistik-ic">{govde}</div></div>"##)
}

/// Demo sayfasının tamamı. Dönen metin tek dosyadır: dış istek yok.
pub fn uret(g: &Girdi) -> String {
    let ic = icerik(g.tur);
    // Sinematik profil kendi sahne motorunu kullanır; ajans ve klasik
    // yolları bu satırdan etkilenmez.
    if g.profil.eq_ignore_ascii_case("sinematik") {
        return sinematik_sayfa(g, &ic);
    }
    // Tipografi çifti: mutfak, güzellik, klinik, otel ve eğlence gibi sıcak
    // sektörlerde başlık serif; mağaza, kurum, ofis ve teknik serviste grotesk.
    // Böylece iki demo yan yana konduğunda da aynı şablon gibi durmuyor.
    // "gece" teması sektör ne olursa olsun serif manşet kullanır: paletin
    // tamamı Instrument Serif'in geniş ritmine göre kuruludur.
    let gece = g.tema.trim() == "gece";
    let isik = g.tema.trim() == "isik";
    // İki lüks tema da aynı sabit sahne mimarisini kullanır; yalnız palet
    // ve yüzeyler değişir.
    let sahne = gece || isik;
    // Ajans profili: sabit tek şablonu bırakır; hero + manifesto + hizmetler +
    // seçili işler + iletişim olmak üzere beş bölümlü sinematik kompozisyon.
    if !sahne && g.profil.eq_ignore_ascii_case("ajans") {
        return ajans_sayfa(g, &ic);
    }
    let serif = sahne
        || matches!(
            g.tur,
            "menu" | "kafe" | "unlu" | "galeri" | "klinik" | "otel" | "eglence"
        );
    let tasarim = g.tasarim.as_ref();
    let hareket = tasarim
        .map(|t| t.hareket.as_str())
        .filter(|metin| !metin.is_empty())
        .unwrap_or("dengeli");
    let tasarim_imza = tasarim
        .map(|t| {
            format!(
                "<!-- tasarim: {} · {} · {} -->
",
                t.ad, t.grup, t.hareket
            )
        })
        .unwrap_or_default();
    let tasarim_baslik = tasarim
        .map(|t| t.baslik.trim().to_string())
        .filter(|metin| !metin.is_empty());
    let govde_yigin = tasarim
        .map(|t| t.govde.trim().to_string())
        .filter(|metin| !metin.is_empty())
        .unwrap_or_else(|| "'Geist',system-ui,-apple-system,'Segoe UI',sans-serif".to_string());
    let baslik_yigin = if sahne {
        "'Cormorant Garamond','Instrument Serif',Georgia,'Times New Roman',serif"
    } else if let Some(yigin) = tasarim_baslik.as_deref() {
        yigin
    } else if serif {
        "'Instrument Serif','Space Grotesk',Georgia,'Times New Roman',serif"
    } else {
        "'Space Grotesk','Geist',system-ui,-apple-system,'Segoe UI',sans-serif"
    };
    let ad = kacis(&g.ad);
    let sektor = kacis(&g.sektor);
    let adres = kacis(&g.adres);
    let bolge = ilce_metni(&g.adres);
    let harf =
        g.ad.chars()
            .next()
            .map(|harf| harf.to_uppercase().to_string())
            .unwrap_or_else(|| "•".to_string());
    let tel_hedef = g
        .telefon
        .as_ref()
        .map(|tel| format!("tel:+{}", tel.trim_start_matches('+')));

    // Panelden yazılan marka cümlesi ve buton metni sektör metnini ezer.
    let slogan = match g.slogan.trim() {
        "" => ic.alt.to_string(),
        metin => metin.to_string(),
    };
    let cta = match g.cta.trim() {
        "" => tasarim
            .map(|t| t.cta.trim())
            .filter(|metin| !metin.is_empty())
            .unwrap_or(ic.cta)
            .to_string(),
        metin => metin.to_string(),
    };
    // Menü/hizmet bölümünün başlığı: üst menü, bölüm ve altbilgi aynı metni
    // kullanır ki üç yerde farklı ad görünmesin.
    let bolum_metni = ozel(&g.metin.bolum_baslik, ic.bolum);

    // Acil sektörler: konum gönderme, canlı rozetler ve yol yardım asistanı.
    let acil = acil_sektor(&g.sektor) || tasarim.is_some_and(|t| t.aciliyet >= 2);
    let wa_no: String = g
        .telefon
        .as_deref()
        .or(g.satici_tel.as_deref())
        .map(|tel| tel.chars().filter(char::is_ascii_digit).collect())
        .unwrap_or_default();
    let konum_mesaj = url_kacis("Merhaba, yolda kaldım. Konumum: ");
    let konum_buton = if acil && !wa_no.is_empty() {
        format!(
            r##"<a class="buton buton-konum" href="https://wa.me/{wa_no}?text={konum_mesaj}" target="_blank" rel="noopener">{KONUM_IKON}Canlı konum gönder</a>"##
        )
    } else {
        String::new()
    };
    let hero_rozetler = if acil {
        let metinler: Vec<String> = tasarim
            .map(|t| t.rozetler.clone())
            .filter(|liste| !liste.is_empty())
            .unwrap_or_else(|| {
                vec![
                    format!("7/24 Aktif {sektor} Ekibi"),
                    "Ortalama Varış: 25 Dk".to_string(),
                    "Konum · tek dokunuş".to_string(),
                ]
            });
        let mut cips = String::new();
        for (sira, metin) in metinler.iter().take(3).enumerate() {
            if sira == 0 {
                cips.push_str(&format!(
                    r##"<span class="hcip canli"><b></b>{}</span>"##,
                    kacis(metin)
                ));
            } else {
                cips.push_str(&format!(r##"<span class="hcip">{}</span>"##, kacis(metin)));
            }
        }
        format!(r##"<div class="hero-rozetler">{cips}</div>"##)
    } else {
        String::new()
    };

    let birincil = match &tel_hedef {
        Some(hedef) => format!(
            r##"<a class="buton buton-dolu" href="{hedef}">{TELEFON_IKON}{}</a>"##,
            kacis(&cta)
        ),
        None => String::new(),
    };
    // Tek ekranlık temada tek çağrı kalır: ikinci buton çizilmez.
    let ikincil = if sahne {
        String::new()
    } else if g.konum.is_some() && g.bolumler.harita {
        format!(
            r##"<a class="buton buton-cizgi" href="#konum">{}</a>"##,
            ozel(&g.ikincil, "Konumu gör")
        )
    } else {
        format!(
            r##"<a class="buton buton-cizgi" href="#iletisim">{}</a>"##,
            ozel(&g.ikincil, "İletişim")
        )
    };

    // Menü/hizmet listesi — her satır gecikmeli açılır. Panelden yazılan
    // satır şablonu ezer; boş bırakılan satır şablonda kalır.
    let mut liste = String::new();
    for (sira, (kalem, aciklama)) in ic.kalemler.iter().enumerate() {
        liste.push_str(&format!(
            r##"<li class="satir" data-rise style="--g:{}">
<div class="satir-ust"><h3>{}</h3></div>
<p>{}</p>
</li>"##,
            sira * 70,
            ozel(g.metin.satir(&g.metin.kalemler, sira, 0), kalem),
            ozel(g.metin.satir(&g.metin.kalemler, sira, 1), aciklama)
        ));
    }

    // Galeri: işletmenin kendi fotoğrafları (en fazla 3). Fotoğraf yoksa
    // bölüm hiç çizilmez; boş kutu göstergesi olmaz.
    let galeri = if g.galeri.is_empty() {
        String::new()
    } else {
        let mut kareler = String::new();
        for (sira, (mime, veri)) in g.galeri.iter().enumerate() {
            kareler.push_str(&format!(
                r##"<figure class="galeri-kare" data-rise style="--g:{}">
<img src="data:{mime};base64,{veri}" alt="{} — fotoğraf {}" loading="lazy">
</figure>"##,
                (sira + 2) * 70,
                ad,
                sira + 2
            ));
        }
        let not = g.galeri_not.as_deref().unwrap_or(if g.galeri_ornek {
            "Örnek kareler · işletmenin kendi fotoğraflarıyla değiştirilir"
        } else {
            "İşletmenin kendi fotoğrafları"
        });
        format!(
            r##"<div class="galeri-ust" data-rise><span class="ince">Galeri</span><span class="galeri-not">{not}</span></div>
<div class="galeri">{kareler}</div>"##
        )
    };

    let mut nedenler = String::new();
    for (sira, (baslik, metin)) in ic.neden.iter().enumerate() {
        nedenler.push_str(&format!(
            r##"<div class="kart" data-rise style="--g:{}">
<span class="sira">0{}</span>
<h3>{}</h3>
<p>{}</p>
</div>"##,
            sira * 90,
            sira + 1,
            kacis(baslik),
            kacis(metin)
        ));
    }

    let mut yorumlar = String::new();
    for (sira, (soz, kim)) in ic.yorumlar.iter().enumerate() {
        let soz = ozel(g.metin.satir(&g.metin.yorumlar, sira, 0), soz);
        let kim = ozel(g.metin.satir(&g.metin.yorumlar, sira, 1), kim);
        let bas_harf = kim.chars().next().unwrap_or('•').to_uppercase();
        yorumlar.push_str(&format!(
            r##"<blockquote class="alinti" data-rise style="--g:{}">
<div class="yildizlar">{YILDIZ}{YILDIZ}{YILDIZ}{YILDIZ}{YILDIZ}</div>
<p>{}</p>
<footer><b>{bas_harf}</b><span>{}</span></footer>
</blockquote>"##,
            sira * 90,
            soz,
            kim
        ));
    }

    let akis_parca = format!("<span>{}</span>", kacis(ic.akis));
    let akis = akis_parca.repeat(8);

    let puan_rozett = match (&g.puan, &g.yorum) {
        (Some(puan), Some(yorum)) => format!(
            r##"<div class="rozet rozet-b"><div class="yildizlar">{YILDIZ}{YILDIZ}{YILDIZ}{YILDIZ}{YILDIZ}</div><div class="rozet-puan"><b>{puan}</b><small>{yorum}</small></div></div>"##
        ),
        (Some(puan), None) => format!(
            r##"<div class="rozet rozet-b"><div class="yildizlar">{YILDIZ}{YILDIZ}{YILDIZ}{YILDIZ}{YILDIZ}</div><b>{puan}</b></div>"##
        ),
        // Puan yoksa ikinci rozet çizilmez: boş kutu göstergesi olmaz.
        _ => String::new(),
    };

    // Sahne: işletmenin fotoğrafı varsa o, yoksa tipografik kompozisyon.
    let sahne_ic = match &g.foto {
        Some((mime, veri)) => format!(
            r##"<img class="sahne-foto" src="data:{mime};base64,{veri}" alt="{ad}" loading="eager">
<div class="sahne-perde"></div>
<div class="sahne-koyu"></div>"##,
            ad = ad
        ),
        None => format!(
            r##"<div class="halkalar"></div>
<div class="monogram-buyuk">{harf}</div>"##,
            harf = kacis(&harf)
        ),
    };

    // Harita yalnız panelden açıksa ve koordinat varsa çizilir.
    let harita_goster = g.bolumler.harita && g.konum.is_some();
    let harita = if harita_goster {
        harita_html(g, &ad)
    } else {
        String::new()
    };
    let iletisim_kart = if harita.is_empty() {
        format!(
            r##"<div class="ilet-kart">{KONUM_IKON}<div><small>Adres</small><b>{adres}</b></div></div>"##
        )
    } else {
        harita
    };
    let tel_kart = match (&tel_hedef, &g.telefon_goster) {
        (Some(hedef), goster) => format!(
            r##"<a class="ilet-kart" href="{hedef}">{TELEFON_IKON}<div><small>Telefon</small><b>{}</b></div></a>"##,
            kacis(goster)
        ),
        _ => String::new(),
    };
    let konum_kart = if harita_goster {
        format!(
            r##"<div class="ilet-kart">{KONUM_IKON}<div><small>Adres</small><b>{adres}</b></div></div>"##
        )
    } else {
        String::new()
    };
    let karne = match &g.karne {
        Some(adres) => format!(
            r##"<a href="{}">Dijital görünürlük karnesi</a>"##,
            kacis(adres)
        ),
        None => String::new(),
    };
    let mobil_cta = if acil && !wa_no.is_empty() {
        let ara = match &tel_hedef {
            Some(hedef) => format!(
                r##"<a class="mobil-ara" href="{hedef}" aria-label="Ara">{TELEFON_IKON}<span>Ara</span></a>"##
            ),
            None => String::new(),
        };
        format!(
            r##"<div class="mobil-bar">{ara}<a class="mobil-konum" href="https://wa.me/{wa_no}?text={konum_mesaj}" target="_blank" rel="noopener">{KONUM_IKON}<span>Canlı Konum Gönder</span></a></div>"##
        )
    } else {
        match &tel_hedef {
            Some(hedef) => format!(
                r##"<a class="mobil-cta" href="{hedef}">{}</a>"##,
                kacis(&cta)
            ),
            None => String::new(),
        }
    };
    let bant_buton = match &tel_hedef {
        Some(hedef) => format!(
            r##"<a class="buton" href="{hedef}">{TELEFON_IKON}{}</a>"##,
            kacis(&cta)
        ),
        None => String::new(),
    };

    // Harita altındaki rezervasyon kartı: sayfayı arayanı aramaya götürür.
    let harita_kart = match &tel_hedef {
        Some(hedef) => format!(
            r##"<div class="harita-kart"><div class="ince">İletişim</div><p style="margin:9px 0 16px;font-size:15px;line-height:1.6;color:var(--ink)">{cta} için arayın; {ad} adına not bırakalım.</p><a class="buton buton-dolu" href="{hedef}">{TELEFON_IKON}{cta}</a></div>"##,
            cta = kacis(&cta),
            ad = ad,
            hedef = hedef
        ),
        None => String::new(),
    };

    // Bölüm görünürlüğü: kapatılan bölüm hiç çizilmez, boş çerçeve kalmaz.
    let galeri = if g.bolumler.galeri {
        galeri
    } else {
        String::new()
    };
    let saatler = if g.bolumler.saatler {
        saatler_html(g)
    } else {
        String::new()
    };
    let iletisim_baslik = if g.bolumler.saatler {
        "Ne zaman gelirseniz?"
    } else {
        "Bize ulaşın"
    };
    let iletisim_etiket = if g.bolumler.saatler {
        "Saatler &amp; iletişim"
    } else {
        "İletişim"
    };
    let yorumlar_bolum = if g.bolumler.yorumlar {
        format!(
            r##"<section class="bolum"><div class="kabuk">
<div class="bolum-ust" data-rise><span class="no">04</span><span class="ince">Yorumlar</span></div>
<div class="yorum-kaydirici" id="yorum-kaydirici">
<div class="yorum-iz" id="yorum-iz">{yorumlar}</div>
<div class="yorum-kontroller">
<button class="yk-btn" type="button" data-yon="-1" aria-label="Önceki yorum">‹</button>
<button class="yk-btn" type="button" data-yon="1" aria-label="Sonraki yorum">›</button>
<div class="yorum-noktalar" id="yorum-noktalar"></div>
</div>
</div>
<p class="ornek-not">Yorumlar örnektir · gerçek yorumlarla değiştirilecek</p>
</div></section>
"##
        )
    } else {
        String::new()
    };
    // Köşe biçimi: "keskin"/"yumusak" gövdeye yazılır, CSS değişkenleri ezer.
    let kose = match g.kose.trim() {
        "keskin" => " data-kose=\"keskin\"",
        "yumusak" => " data-kose=\"yumusak\"",
        _ => "",
    };
    let tema_attr = if gece {
        " data-tema=\"gece\""
    } else if isik {
        " data-tema=\"isik\""
    } else {
        ""
    };
    // Üst menü bağlantıları bölüm kimlikleriyle birebir eşleşir; iki
    // temada da ölü bağlantı kalmaz.
    let gece_nav = r##"<a href="#hikaye">Hikâye</a><a href="#menu">{bolum_metni}</a><a href="#iletisim">İletişim</a><a href="#gece-form">Rezervasyon</a>"##;
    let (nav_linkleri, mobil_linkleri, mobil_dugme) = if sahne {
        (
            gece_nav.replace("{bolum_metni}", &bolum_metni),
            gece_nav.replace("{bolum_metni}", &bolum_metni),
            r##"<button class="mobil-dugme" id="mobil-dugme" aria-expanded="false" aria-label="Menü">☰</button>"##.to_string(),
        )
    } else {
        (
            format!(r##"<a href="#hakkimizda">Hakkında</a><a href="#bolum">{bolum_metni}</a><a href="#neden">Neden biz</a><a href="#iletisim">İletişim</a>"##),
            format!(r##"<a href="#hakkimizda">Hakkında</a><a href="#bolum">{bolum_metni}</a><a href="#neden">Neden biz</a><a href="#iletisim">İletişim</a>"##),
            r##"<button class="mobil-dugme" id="mobil-dugme" aria-expanded="false" aria-label="Menü">☰</button>"##.to_string(),
        )
    };

    let mut sayfa = String::new();
    // Açık zeminde gölge/harita/ışık katmanlarının yönü değişir.
    let tema_acik = parlak_mi(&g.zemin);
    sayfa.push_str(if tema_acik {
        "<!DOCTYPE html>\n<html lang=\"tr\" class=\"tema-acik\">\n<head>\n<meta charset=\"utf-8\">\n"
    } else {
        "<!DOCTYPE html>\n<html lang=\"tr\">\n<head>\n<meta charset=\"utf-8\">\n"
    });
    sayfa.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    sayfa.push_str(&format!("<title>{ad} — {sektor} | {bolge}</title>\n"));
    let ozet = if adres.is_empty() {
        format!("{sektor} · {ad} · {}", ic.alt)
    } else {
        format!("{sektor} · {adres} · {}", ic.alt)
    };
    sayfa.push_str(&format!(
        concat!(
            "<meta name=\"description\" content=\"{ozet}\">\n",
            "<meta property=\"og:type\" content=\"website\">\n",
            "<meta property=\"og:site_name\" content=\"{ad}\">\n",
            "<meta property=\"og:title\" content=\"{ad} — {sektor}\">\n",
            "<meta property=\"og:description\" content=\"{ozet}\">\n",
            "<meta property=\"og:locale\" content=\"tr_TR\">\n",
            "<meta name=\"twitter:card\" content=\"summary\">\n",
            "<meta name=\"theme-color\" content=\"{vurgu}\">\n",
            "<meta name=\"robots\" content=\"noindex\">\n"
        ),
        ad = ad,
        sektor = sektor,
        ozet = kacis(&ozet),
        vurgu = g.vurgu
    ));
    sayfa.push_str("<style>\n");
    sayfa.push_str(g.font_css);
    sayfa.push_str(&format!(
        ":root{{--accent:{vurgu};--accent-2:{vurgu2};--accent-karsi:{karsi};--bg:{zemin};--ink:{yazi};--r-1:8px;--r-2:14px;--r-3:18px;--r-4:26px;--f-baslik:{baslik};--f-govde:{govde}}}\n",
        vurgu = g.vurgu,
        zemin = g.zemin,
        yazi = g.yazi,
        baslik = baslik_yigin,
        govde = govde_yigin,
        vurgu2 = tasarim
            .map(|t| t.vurgu2.trim())
            .filter(|metin| !metin.is_empty())
            .unwrap_or(g.vurgu.as_str()),
        karsi = if parlak_mi(&g.vurgu) { "#12100e" } else { "#f8f5f0" }
    ));
    sayfa.push_str(CSS);
    // Köşe biçimi: ölçüler CSS değişkenlerinde, burada yalnız ölçek değişir.
    sayfa.push_str(
        "body[data-kose=\"keskin\"]{--r-1:2px;--r-2:3px;--r-3:4px;--r-4:5px}\n\
         body[data-kose=\"yumusak\"]{--r-1:12px;--r-2:22px;--r-3:28px;--r-4:40px}\n",
    );
    if gece {
        sayfa.push_str(GECE_CSS);
    } else if isik {
        // Aynı sahne kuralları açık palete çevrilir; karanlık değerler
        // ISIK_CSS içinde tek tek ezilir.
        sayfa.push_str(&GECE_CSS.replace("data-tema=gece", "data-tema=isik"));
        sayfa.push_str(ISIK_CSS);
    }
    sayfa.push_str(EK_CSS);
    // JS kapalıysa içerik görünür kalır: açılım animasyonu beklemesin.
    sayfa.push_str(&format!(
        "</style>\n<noscript><style>[data-rise]{{opacity:1;transform:none}}.manset .kelime i{{transform:none}}.kelime i{{transform:none;opacity:1;filter:none}}.klm i{{transform:none;opacity:1;filter:none}}.fasil{{position:relative;opacity:1;transform:none;pointer-events:auto}}.perde{{height:auto}}.perde-sabit{{position:relative;height:auto}}.bolum-ust::before{{transform:scaleX(1)}}.sahne{{animation:none}}</style></noscript>\n</head>\n<body{kose}{tema_attr} data-yazi=\"{yazi_adi}\">\n",
        yazi_adi = if serif { "serif" } else { "grotesk" }
    ));
    sayfa.push_str(&format!(
        r##"<div class="ilerleme" id="ilerleme" aria-hidden="true"></div>
<div class="serit">Taslak demo · {ad} için hazırlandı · içerik taslaktır</div>
<header class="ust" id="ust">
<div class="ust-ic">
<a class="marka" href="#ust"><span class="monogram">{harf}</span><span>{ad}</span></a>
<nav class="menuler">{nav_linkleri}</nav>
{tel_dugme}
{mobil_dugme}
</div>
<nav class="mobil-menu" id="mobil-menu">{mobil_linkleri}</nav>
</header>
"##,
        ad = ad,
        harf = kacis(&harf),
        nav_linkleri = nav_linkleri,
        mobil_dugme = mobil_dugme,
        mobil_linkleri = mobil_linkleri,
        tel_dugme = match &tel_hedef {
            Some(hedef) => format!(
                r##"<a class="ust-tel" href="{hedef}">{TELEFON_IKON}{}</a>"##,
                kacis(&g.telefon_goster)
            ),
            None => String::new(),
        }
    ));

    // Hero — manşet kelime kelime açılır; ikinci satır kaldığı yerden devam eder.
    let (satir1, kelime_sayaci) = manset_satiri(&ozel(&g.metin.manset1, ic.manset[0]), 0);
    let (satir2, _) = manset_satiri(&ozel(&g.metin.manset2, ic.manset[1]), kelime_sayaci);
    if !sahne {
        sayfa.push_str(&format!(
            r##"<section class="hero">
<div class="kabuk hero-ic">
<div class="heroi">
<span class="rozet-cip"><b></b>{sektor} · {bolge}</span>
<h1 class="manset">
{satir1}
{satir2}
</h1>
<p class="hero-alt">{alt}</p>
{hero_rozetler}
<div class="hero-butonlar">{birincil}{ikincil}{konum_buton}</div>
<div class="durum"><span class="nokta-canli" id="durum-nokta"></span><span class="durum-metin" id="durum-metin">Çalışma saatleri yükleniyor…</span></div>
</div>
<div class="{sahne_sinif}" data-par="0.05">
{sahne_ic}
<div class="sahne-alt"><span>{sektor}</span><span>{bolge}</span></div>
<div class="rozet rozet-a"><small>Bugün</small><b id="rozet-saat">—</b></div>
{puan_rozett}
</div>
</div>
{istatistik}
</section>
<div class="akis"><div class="akis-ic">{akis}{akis}</div></div>
"##,
            sektor = sektor,
            satir1 = satir1,
            satir2 = satir2,
            alt = kacis(&slogan),
            birincil = birincil,
            ikincil = ikincil,
            hero_rozetler = hero_rozetler,
            konum_buton = konum_buton,
            sahne_sinif = if g.foto.is_some() { "sahne foto" } else { "sahne" },
            sahne_ic = sahne_ic,
            bolge = bolge,
            puan_rozett = puan_rozett,
            istatistik = istatistik_html(g, &bolum_metni),
            akis = akis
        ));
    }

    // Sinematik kaydırma: iki tam genişlik kare, yavaş zoom + parallax ve
    // sırayla süzülen başlık. Fotoğraf yoksa gradyan "doku" karesi çizilir.
    let sinema = if !sahne && tasarim.is_some_and(|t| t.sinematik) {
        let kareler: Vec<&FotoVerisi> = g.foto.iter().chain(g.galeri.iter()).take(2).collect();
        let mut govde = String::new();
        let sinema_baslik = ozel(&g.metin.hakkinda_baslik, ic.hikaye_baslik);
        let sinema_metin = ozel(&g.metin.hakkinda1, ic.hikaye[0]);
        let bolumler = [
            (sinema_baslik.as_str(), sinema_metin.as_str()),
            (bolum_metni.as_str(), ic.bolum_alt),
        ];
        for (sira, (baslik, metin)) in bolumler.iter().enumerate() {
            let gorsel = match kareler.get(sira) {
                Some((mime, veri)) => format!(
                    r##"<div class="sinema-ic"><img src="data:{mime};base64,{veri}" alt="{ad} — {sektor}" loading="lazy" decoding="async"><div class="sinema-perde"></div></div>"##
                ),
                None => String::from(r##"<div class="sinema-ic doku"></div>"##),
            };
            govde.push_str(&format!(
                r##"<div class="sinema-kare" data-sinema style="--g:{gecikme}">
{gorsel}
<div class="sinema-yazi" data-rise style="--g:{gecikme}">
<span class="ince">{no} — {sektor} · {bolge}</span>
<h2>{baslik}</h2>
<p>{metin}</p>
</div>
</div>"##,
                gecikme = sira * 120,
                no = sira + 1
            ));
        }
        format!(
            r##"<section class="sinema" aria-label="Sinematik akış"><div class="kabuk sinema-kabuk">{govde}</div></section>"##
        )
    } else {
        String::new()
    };

    // Hakkında: fotoğraf varsa iki sütunlu, görselli düzen; yoksa başlık ve
    // metin ikiye ayrılır. Aynı fotoğraf galeride tekrar etmesin diye kare
    // galeriden ayrılıp buraya verilir (bkz. `demo_html`).
    let hikaye_baslik = ozel(&g.metin.hakkinda_baslik, ic.hikaye_baslik);
    let hikaye1 = ozel(&g.metin.hakkinda1, ic.hikaye[0]);
    let hikaye2 = ozel(&g.metin.hakkinda2, ic.hikaye[1]);
    let hikaye = match &g.hikaye_foto {
        Some((mime, veri)) => format!(
            r##"<div class="hikaye fotolu">
<div class="hikaye-metin">
<h2 data-rise style="--g:60">{baslik}</h2>
<p class="hikaye-buyuk" data-rise style="--g:140">{p1}</p>
<p data-rise style="--g:200">{p2}</p>
</div>
<figure class="hikaye-gorsel" data-rise style="--g:120"><img src="data:{mime};base64,{veri}" alt="{ad} — {sektor}" loading="lazy" decoding="async"></figure>
</div>"##,
            mime = mime,
            veri = veri,
            ad = ad,
            sektor = sektor,
            baslik = hikaye_baslik,
            p1 = hikaye1,
            p2 = hikaye2
        ),
        None => format!(
            r##"<div class="hikaye">
<div><h2 data-rise style="--g:60">{baslik}</h2></div>
<div>
<p class="hikaye-buyuk" data-rise style="--g:120">{p1}</p>
<p data-rise style="--g:180">{p2}</p>
</div>
</div>"##,
            baslik = hikaye_baslik,
            p1 = hikaye1,
            p2 = hikaye2
        ),
    };

    // Satıcı bandı: yalnız gönderen numarası tanımlıysa. Taslağı beğenen
    // işletme buradan tek dokunuşla satıcıya yazıyor; gelen mesaj işletme
    // adını taşıdığı için hangi demo olduğu karışmıyor.
    let satici_bandi = match g.satici_tel.as_deref().map(str::trim) {
        Some(tel) if !tel.is_empty() => {
            let mesaj = url_kacis(&format!(
                "Merhaba, {} için hazırladığınız demo hakkında yazıyorum.",
                g.ad
            ));
            format!(
                r##"<section class="satici"><div class="kabuk satici-ic">
<div><span class="ince">Bu bir taslak</span><h2>Bu taslağı size özel hale getirelim.</h2><p>Renkler, fotoğraflar ve metinler birlikte düzenlenir; yayına alınması bir gün sürer.</p></div>
<a class="buton buton-cizgi" href="https://wa.me/{tel}?text={mesaj}" target="_blank" rel="noopener" style="border-color:color-mix(in srgb,var(--accent) 55%,transparent);color:var(--accent-metin)">{TELEFON_IKON}WhatsApp'tan yazın</a>
</div></section>"##
            )
        }
        _ => String::new(),
    };

    // Hakkında + listeler + neden biz + yorumlar
    // "Gece" teması tek ekranlık: bu bölümler hiç çizilmez, boş çerçeve kalmaz.
    // "Gece" teması: alt alta bölüm yok. Tek bir sabit (sticky) sahne var;
    // kaydırma ilerledikçe dört fasıl sırayla açılır ve arka plandaki
    // fotoğraflar çapraz geçişle değişir. Fasıl kimlikleri menüyle birebir.
    let wa_hedef: String = g
        .telefon
        .as_deref()
        .or(g.satici_tel.as_deref())
        .map(|tel| tel.chars().filter(char::is_ascii_digit).collect())
        .unwrap_or_default();
    if sahne {
        let bolum_kelimeli = kelimeler(&bolum_metni);
        let iletisim_baslik_kelimeli = kelimeler(iletisim_baslik);
        let masa_kelimeli = kelimeler("Masa ayırt.");
        let katmanlar: Vec<&FotoVerisi> = g.foto.iter().chain(g.galeri.iter()).take(4).collect();
        let mut perde_kareler = String::new();
        if katmanlar.is_empty() {
            for _ in 0..4 {
                perde_kareler.push_str(r##"<div class="perde-kare doku"></div>"##);
            }
        } else {
            for (sira, kare) in katmanlar.iter().enumerate() {
                let (mime, veri) = *kare;
                perde_kareler.push_str(&format!(
                    r##"<div class="perde-kare{sinif}"><img src="data:{mime};base64,{veri}" alt="" loading="lazy" decoding="async"></div>"##,
                    sinif = if sira == 0 { " gorunur" } else { "" }
                ));
            }
        }
        sayfa.push_str(&format!(
            r##"<section class="perde" data-perde>
<div class="perde-sabit">
<div class="perde-arka" aria-hidden="true">
{perde_kareler}
<div class="perde-perde"></div>
<div class="sinema-kor"><canvas id="kor"></canvas></div>
</div>

<div class="perde-ic kabuk">
<div class="fasil" id="hikaye" data-fasil="0">
<div class="fasil-sol">
<span class="fasil-no">01 — {sektor} · {bolge}</span>
<h1 class="manset gece-manset">
{satir1}
{satir2}
</h1>
<p class="hero-alt">{alt}</p>
{hero_rozetler}
<div class="hero-butonlar">{birincil}<a class="buton buton-cizgi" href="#menu">Menüyü gör</a>{konum_buton}</div>
</div>
<div class="fasil-sag">{hikaye}</div>
</div>

<div class="fasil" id="menu" data-fasil="1">
<div class="fasil-sol">
<span class="fasil-no">02 — {bolum_etiket}</span>
<h2 class="gece-baslik">{bolum_kelimeli}</h2>
<p class="gece-not">{bolum_alt}</p>
</div>
<div class="fasil-sag"><ul class="liste gece-liste">{liste}</ul></div>
</div>

<div class="fasil" id="iletisim" data-fasil="2">
<div class="fasil-sol">
<span class="fasil-no">03 — {iletisim_etiket}</span>
<h2 class="gece-baslik">{iletisim_baslik_kelimeli}</h2>
<div class="gece-bilgi">{saatler}{tel_kart}{konum_kart}</div>
</div>
<div class="fasil-sag"><div class="gece-harita">{harita_bolum}</div></div>
</div>

<div class="fasil" id="gece-form" data-fasil="3">
<div class="fasil-sol">
<span class="fasil-no">04 — Rezervasyon</span>
<h2 class="gece-baslik">{masa_kelimeli}</h2>
<p class="gece-not">Aynı gün rezervasyon için telefon en hızlısı. Formu doldurursanız WhatsApp'tan dönüyoruz; masa adınıza ayrılır.</p>
</div>
<div class="fasil-sag">
<form class="gece-form" id="rezervasyon-form" novalidate>
<div class="alan"><label for="gece-ad">Adınız</label><input id="gece-ad" name="ad" type="text" autocomplete="name" required></div>
<div class="alan"><label for="gece-tel">Telefon</label><input id="gece-tel" name="tel" type="tel" inputmode="tel" autocomplete="tel" required></div>
<div class="alan"><label for="gece-mesaj">Kaç kişi, ne zaman?</label><textarea id="gece-mesaj" name="mesaj" rows="2"></textarea></div>
<button class="buton buton-dolu" type="submit">Rezervasyon gönder</button>
<p class="form-not" id="gece-form-not" role="status" aria-live="polite"></p>
</form>
</div>
</div>
</div>

<div class="perde-ray" aria-hidden="true">
<span>01</span>
<span class="ray-cizgi"><i></i></span>
<span>04</span>
</div>
</div>

<script>
(function () {{
  var form = document.getElementById("rezervasyon-form");
  if (!form) return;
  var not = document.getElementById("gece-form-not");
  var hedef = "{wa_hedef}";
  form.addEventListener("submit", function (e) {{
    e.preventDefault();
    var ad = form.ad.value.trim(), tel = form.tel.value.trim(), mesaj = form.mesaj.value.trim();
    if (!ad || !tel) {{ if (not) not.textContent = "Ad ve telefon gerekli."; return; }}
    if (!hedef) {{ if (not) not.textContent = "Bu bir taslaktır: form yayına alınınca işletmeye iletilir."; return; }}
    var metin = "Rezervasyon: " + ad + " (" + tel + "). " + (mesaj || "Masa ayırtmak istiyorum.");
    window.open("https://wa.me/" + hedef + "?text=" + encodeURIComponent(metin), "_blank", "noopener");
    if (not) not.textContent = "WhatsApp açılıyor…";
  }});
}})();
</script>
</section>
"##,
            perde_kareler = perde_kareler,
            sektor = sektor,
            bolge = bolge,
            satir1 = satir1,
            satir2 = satir2,
            alt = kacis(&slogan),
            birincil = birincil,
            hero_rozetler = hero_rozetler,
            konum_buton = konum_buton,
            hikaye = hikaye,
            bolum_etiket = ozel(&g.metin.bolum_etiket, ic.etiket),
            bolum_alt = ozel(&g.metin.bolum_alt, ic.bolum_alt),
            liste = liste,
            iletisim_etiket = iletisim_etiket,
            saatler = saatler,
            tel_kart = tel_kart,
            konum_kart = konum_kart,
            harita_bolum = iletisim_kart,
            wa_hedef = wa_hedef,
            bolum_kelimeli = bolum_kelimeli,
            iletisim_baslik_kelimeli = iletisim_baslik_kelimeli,
            masa_kelimeli = masa_kelimeli
        ));
    }

    if !sahne {
        sayfa.push_str(&sinema);
        sayfa.push_str(&format!(
        r##"<section class="bolum" id="hakkimizda"><div class="kabuk">
<div class="bolum-ust" data-rise><span class="no">01</span><span class="ince">Hakkında</span></div>
{hikaye}
</div></section>

<section class="bolum" id="bolum"><div class="kabuk">
<div class="bolum-ust" data-rise><span class="no">02</span><span class="ince">{bolum_etiket}</span></div>
<h2 data-rise style="--g:60">{bolum}</h2>
<p class="hero-alt" data-rise style="--g:110;margin-top:12px">{bolum_alt}</p>
<ul class="liste" data-rise style="--g:150;margin-top:34px">{liste}</ul>
<p class="liste-alt">Liste, işletmenin gerçek hizmetleriyle değiştirilecek.</p>
{galeri}
</div></section>

<section class="bolum" id="neden"><div class="kabuk">
<div class="bolum-ust" data-rise><span class="no">03</span><span class="ince">Neden biz</span></div>
<div class="uc">{nedenler_kart}</div>
</div></section>

{yorumlar_bolum}
"##,
        hikaye = hikaye,
        bolum_etiket = ozel(&g.metin.bolum_etiket, ic.etiket),
        bolum = bolum_metni,
        bolum_alt = ozel(&g.metin.bolum_alt, ic.bolum_alt),
        liste = liste,
        galeri = galeri,
        nedenler_kart = nedenler,
        yorumlar_bolum = yorumlar_bolum
    ));
    }

    // Altbilgi notu: işletme fotoğrafı ve varsa örnek karelerin atıfları.
    // Kaynak zaten "Fotoğraflar:" ile başlıyorsa ikinci kez önek eklenmez;
    // iki alan aynı listeyi taşıyorsa atıf bir kez yazılır.
    let atif_cumlesi = |atif: &Option<String>| -> String {
        match atif {
            Some(atif) if !atif.trim().is_empty() => {
                let metin = kacis(atif.trim());
                if metin.starts_with("Fotoğraf") {
                    format!(" {metin}.")
                } else {
                    format!(" Fotoğraf: {metin}.")
                }
            }
            _ => String::new(),
        }
    };
    let foto_not = if g.foto_atif == g.galeri_atif {
        atif_cumlesi(&g.foto_atif)
    } else {
        format!(
            "{}{}",
            atif_cumlesi(&g.foto_atif),
            atif_cumlesi(&g.galeri_atif)
        )
    };

    if !sahne {
        sayfa.push_str(&format!(
        r##"<section class="bolum" id="iletisim"><div class="kabuk">
<div class="bolum-ust" data-rise><span class="no">05</span><span class="ince">{iletisim_etiket}</span></div>
<div class="iletisim">
<div>
<h2 data-rise style="--g:50">{iletisim_baslik}</h2>
<div data-rise style="--g:110;margin-top:24px">{saatler}</div>
<div class="ilet-kartlar">{tel_kart}{konum_kart}</div>
</div>
<div id="konum" data-rise style="--g:80">{harita_bolum}{harita_kart}</div>
</div>
</div></section>

<section class="bant"><div class="kabuk bant-ic">
<div><h2>{cta} — bir telefon kadar yakın.</h2><p>{ad} · {bolge}</p></div>
{bant_buton}
</div></section>
{satici_bandi}

<footer class="altbilgi"><div class="kabuk alt-ic">
<div>
<div class="alt-marka"><span class="monogram">{harf}</span>{ad}</div>
<p class="alt-not">Bu taslak {gonderen} tarafından {ad} için hazırlandı. Yorumlar örnektir; adres ve saat bilgileri {kaynak} kaydından alınmıştır.{foto_not}</p>
</div>
<div class="alt-baglar"><a href="#hakkimizda">Hakkında</a><a href="#bolum">{bolum}</a><a href="#iletisim">İletişim</a>{karne}</div>
</div></footer>
{mobil_cta}
<div id="fener" aria-hidden="true"></div>
<div class="gren" aria-hidden="true"></div>
"##,
        cta = kacis(&cta),
        ad = ad,
        bolge = bolge,
        bant_buton = bant_buton,
        satici_bandi = satici_bandi,
        gonderen = kacis(&g.gonderen),
        kaynak = kacis(&g.kaynak),
        foto_not = foto_not,
        harf = kacis(&harf),
        bolum = bolum_metni,
        karne = karne,
        mobil_cta = mobil_cta,
        saatler = saatler,
        iletisim_etiket = iletisim_etiket,
        iletisim_baslik = iletisim_baslik,
        tel_kart = tel_kart,
        konum_kart = konum_kart,
        harita_bolum = iletisim_kart,
        harita_kart = harita_kart
    ));
    }

    // Tek ekranlık tema: iletişim ve kapanış bandı yerine sade bir altbilgi.
    // Satıcı bandı kalır: taslağı beğenen işletme buradan yazar.
    if sahne {
        sayfa.push_str(&format!(
            r##"{satici_bandi}
<footer class="altbilgi"><div class="kabuk alt-ic">
<div>
<div class="alt-marka"><span class="monogram">{harf}</span>{ad}</div>
<p class="alt-not">Bu taslak {gonderen} tarafından {ad} için hazırlandı. Adres ve iletişim bilgileri {kaynak} kaydından alınmıştır.{foto_not}</p>
</div>
<div class="alt-baglar">{karne}</div>
</div></footer>
{mobil_cta}
<div id="fener" aria-hidden="true"></div>
<div class="gren" aria-hidden="true"></div>
"##,
            ad = ad,
            harf = kacis(&harf),
            gonderen = kacis(&g.gonderen),
            kaynak = kacis(&g.kaynak),
            foto_not = foto_not,
            karne = karne,
            mobil_cta = mobil_cta,
            satici_bandi = satici_bandi
        ));
    }

    let asistan = if acil && !wa_no.is_empty() {
        format!(
            r##"<div class="asistan" id="asistan" data-wa="{wa_no}">
<button class="asistan-dugme" id="asistan-dugme" type="button" aria-expanded="false" aria-controls="asistan-panel"><span class="asistan-nokta"></span>Yol Yardım Asistanı</button>
<div class="asistan-panel" id="asistan-panel" hidden>
<div class="asistan-ust"><b>Yol Yardım Asistanı</b><button class="asistan-kapat" id="asistan-kapat" type="button" aria-label="Kapat">×</button></div>
<p class="asistan-soru">Durumunuzu seçin, en yakın ekibi yönlendirelim.</p>
<div class="asistan-secim" role="group" aria-label="Acil durum">
<button type="button" data-durum="Arıza">🔧 Arıza</button>
<button type="button" data-durum="Kaza">🚨 Kaza</button>
<button type="button" data-durum="Lastik">🛞 Lastik</button>
<button type="button" data-durum="Akü">🔋 Akü</button>
</div>
<label class="asistan-alan"><span>Konum notu (isteğe bağlı)</span><input id="asistan-not-girdi" type="text" placeholder="Örn. otoban çıkışı, benzinlik yanı" autocomplete="off"></label>
<button class="asistan-gonder" id="asistan-gonder" type="button">Konumumu gönder</button>
<p class="asistan-not" id="asistan-not" role="status" aria-live="polite"></p>
</div>
</div>"##
        )
    } else {
        String::new()
    };

    // Demo süresi: ilk açılıştan 12 saat sonra sayfa kapanır.
    sayfa.push_str(
        "<div class=\"suresi-doldu\" id=\"suresi-doldu\" hidden>\n<p>Bu demonun süresi doldu.<br>Güncel bir demo için işletmeyle iletişime geçin.</p>\n</div>\n",
    );
    sayfa.push_str(&asistan);
    let program = g.program.clone().unwrap_or_else(|| "null".to_string());
    sayfa.push_str(&SCRIPTS.replace("__PROGRAM__", &program));
    sayfa.push_str(&EK_JS.replace("__WA__", &wa_no));
    sayfa.push_str(&format!(
        concat!(
            "<script>\n",
            "(function(){{\n",
            "  var a='demo_{anahtar}', b=Number(localStorage.getItem(a)||Date.now());\n",
            "  localStorage.setItem(a,b);\n",
            "  var sure=document.getElementById('suresi-doldu');\n",
            "  if(sure && Date.now()-b>12*60*60*1000){{sure.hidden=false;document.body.style.overflow='hidden';}}\n",
            "}})();\n",
            "</script>\n</body>\n</html>\n"
        ),
        anahtar = g.anahtar
    ));
    let imza = if tasarim_imza.is_empty() {
        String::new()
    } else {
        tasarim_imza
    };
    sayfa.replace("<body", &format!("{imza}<body data-hareket=\"{hareket}\""))
}

/// Sayfanın davranışı: açılım, parallax, canlı açık/kapalı durumu, mobil menü.
const SCRIPTS: &str = r##"<script>
(function () {
  var PROGRAM = __PROGRAM__;
  var GUNLER = ["Pazartesi","Salı","Çarşamba","Perşembe","Cuma","Cumartesi","Pazar"];
  var azalt = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function iki(sayi) { return String(sayi).padStart(2, "0"); }
  function saatMetni(dk) { return iki(Math.floor(dk / 60) % 24) + ":" + iki(dk % 60); }
  function kalanMetni(dk) {
    var gun = Math.floor(dk / 1440), saat = Math.floor((dk % 1440) / 60), d = dk % 60;
    var p = [];
    if (gun > 0) p.push(gun + " gün");
    if (saat > 0) p.push(saat + " sa");
    if (d > 0 && gun === 0) p.push(d + " dk");
    return p.length ? p.join(" ") : "0 dk";
  }

  // 1) Üst menü daralması + okuma ilerlemesi
  var ust = document.getElementById("ust");
  var ilerleme = document.getElementById("ilerleme");
  function kaydirma() {
    if (ust) ust.classList.toggle("kucuk", window.scrollY > 12);
    if (ilerleme) {
      var toplam = document.documentElement.scrollHeight - window.innerHeight;
      var oran = toplam > 0 ? Math.min(1, window.scrollY / toplam) : 0;
      ilerleme.style.width = (oran * 100).toFixed(2) + "%";
    }
  }
  kaydirma();
  window.addEventListener("scroll", kaydirma, { passive: true });

  // 2) Mobil menü
  var dugme = document.getElementById("mobil-dugme"), menu = document.getElementById("mobil-menu");
  if (dugme && menu) {
    dugme.addEventListener("click", function () {
      var acik = menu.classList.toggle("acik");
      dugme.setAttribute("aria-expanded", acik ? "true" : "false");
    });
    menu.querySelectorAll("a").forEach(function (a) {
      a.addEventListener("click", function () { menu.classList.remove("acik"); });
    });
  }

  // 3) Kaydırmayla açılan bölümler
  var ogeler = [].slice.call(document.querySelectorAll("[data-rise]"));
  if (!azalt && "IntersectionObserver" in window) {
    var gozcu = new IntersectionObserver(function (girisler) {
      girisler.forEach(function (giris) {
        if (!giris.isIntersecting) return;
        giris.target.classList.add("in");
        gozcu.unobserve(giris.target);
      });
    }, { threshold: 0.12, rootMargin: "0px 0px -6% 0px" });
    ogeler.forEach(function (oge) { gozcu.observe(oge); });
  } else {
    ogeler.forEach(function (oge) { oge.classList.add("in"); });
  }

  // 4) Parallax
  var parlar = [].slice.call(document.querySelectorAll("[data-par]"));
  if (!azalt && parlar.length && window.matchMedia("(hover: hover)").matches) {
    var bekliyor = false;
    function parala() {
      bekliyor = false;
      parlar.forEach(function (oge) {
        var kutu = oge.getBoundingClientRect();
        var oran = (kutu.top + kutu.height / 2 - window.innerHeight / 2) * -Number(oge.dataset.par || 0);
        oge.style.transform = "translate3d(0," + oran.toFixed(1) + "px,0)";
      });
    }
    window.addEventListener("scroll", function () {
      if (bekliyor) return;
      bekliyor = true;
      requestAnimationFrame(parala);
    }, { passive: true });
    parala();
  }

  // 5) Canlı açık/kapalı durumu + bugünün saati
  var metin = document.getElementById("durum-metin");
  var nokta = document.getElementById("durum-nokta");
  var rozet = document.getElementById("rozet-saat");
  var istBugun = document.getElementById("ist-bugun");
  var gunler = document.querySelectorAll(".saat-satir[data-gun]");

  function bugunAralik(gun) {
    var araliklar = (PROGRAM && PROGRAM[gun]) || [];
    if (!araliklar.length) return null;
    return araliklar[0];
  }

  function durumYaz(metinDeger, durum) {
    if (metin) { metin.textContent = metinDeger; metin.className = "durum-metin " + durum; }
    if (nokta) {
      nokta.className = "nokta-canli" + (durum === "kapali" ? " kapali" : (durum === "yakin" ? " yakin" : ""));
    }
  }

  function durumHesapla() {
    if (!PROGRAM) {
      if (metin && !metin.textContent.trim()) metin.textContent = "Çalışma saatleri için arayın";
      return;
    }
    var simdi = new Date();
    var gun = (simdi.getDay() + 6) % 7;
    var dk = simdi.getHours() * 60 + simdi.getMinutes();
    var aralik = bugunAralik(gun);

    if (rozet) rozet.textContent = aralik ? (saatMetni(aralik[0]) + " – " + saatMetni(aralik[1])) : "Kapalı";
    if (istBugun) istBugun.textContent = aralik ? (saatMetni(aralik[0]) + "–" + saatMetni(aralik[1])) : "Kapalı";
    gunler.forEach(function (satir) {
      satir.classList.toggle("bugun", Number(satir.dataset.gun) === gun);
    });

    for (var i = 0; i < (PROGRAM[gun] || []).length; i++) {
      var bas = PROGRAM[gun][i][0], son = PROGRAM[gun][i][1];
      if (dk >= bas && dk < son) {
        var kalan = son - dk;
        if (kalan <= 15) {
          durumYaz("Kapanmak üzere · kapanış " + saatMetni(son) + " · " + kalanMetni(kalan) + " kaldı", "yakin");
        } else {
          durumYaz("Şu an açık · kapanış " + saatMetni(son) + " · " + kalanMetni(kalan) + " kaldı", "acik");
        }
        return;
      }
    }
    for (var g = 0; g < 7; g++) {
      var hedef = (gun + g) % 7;
      var liste = PROGRAM[hedef] || [];
      for (var k = 0; k < liste.length; k++) {
        var acilis = liste[k][0];
        if (g === 0 && acilis <= dk) continue;
        var etiket = g === 0 ? "bugün" : (g === 1 ? "yarın" : GUNLER[hedef].toLocaleLowerCase("tr"));
        var kalanDk = g * 1440 + acilis - dk;
        durumYaz("Şu an kapalı · açılış " + etiket + " " + saatMetni(acilis) + " · " + kalanMetni(kalanDk) + " kaldı", "kapali");
        return;
      }
    }
    durumYaz("Şu an kapalı", "kapali");
  }
  durumHesapla();
  if (PROGRAM) setInterval(durumHesapla, 60000);

  // 6) İmleci izleyen ışık: hedefe anında atlamaz, arkasından yumuşakça gelir
  //    (yalnızca fare olan cihazlarda).
  // Manyetik butonlar: imleç yaklaşınca düğme hafifçe çekilir.
  var mknat = document.querySelectorAll("[data-perde] .buton");
  if (mknat.length && !azalt && window.matchMedia("(hover: hover) and (pointer: fine)").matches) {
    Array.prototype.forEach.call(mknat, function (dugme) {
      dugme.addEventListener("pointermove", function (e) {
        var k = dugme.getBoundingClientRect();
        var x = (e.clientX - k.left - k.width / 2) / (k.width / 2);
        var y = (e.clientY - k.top - k.height / 2) / (k.height / 2);
        dugme.style.transform = "translate(" + (x * 6).toFixed(1) + "px," + (y * 4).toFixed(1) + "px)";
      });
      dugme.addEventListener("pointerleave", function () {
        dugme.style.transform = "";
      });
    });
  }

  var fener = document.getElementById("fener");
  if (fener && !azalt && window.matchMedia("(hover: hover) and (pointer: fine)").matches) {
    var fx = window.innerWidth / 2, fy = window.innerHeight / 3;
    var gx = fx, gy = fy, calisiyor = false;
    function izle() {
      gx += (fx - gx) * 0.12;
      gy += (fy - gy) * 0.12;
      fener.style.setProperty("--fx", gx.toFixed(1) + "px");
      fener.style.setProperty("--fy", gy.toFixed(1) + "px");
      if (Math.abs(fx - gx) < 0.4 && Math.abs(fy - gy) < 0.4) { calisiyor = false; return; }
      requestAnimationFrame(izle);
    }
    window.addEventListener("pointermove", function (olay) {
      if (olay.pointerType === "touch") return;
      fx = olay.clientX; fy = olay.clientY;
      fener.classList.add("acik");
      if (calisiyor) return;
      calisiyor = true;
      requestAnimationFrame(izle);
    }, { passive: true });
    document.addEventListener("pointerleave", function () { fener.classList.remove("acik"); });
  }

  // 7) Manyetik buton: imleç butona yaklaşınca birkaç piksel kayar.
  var miknatis = [].slice.call(document.querySelectorAll(".buton-dolu"));
  if (!azalt && miknatis.length && window.matchMedia("(hover: hover) and (pointer: fine)").matches) {
    miknatis.forEach(function (buton) {
      buton.addEventListener("pointermove", function (olay) {
        var k = buton.getBoundingClientRect();
        var dx = (olay.clientX - (k.left + k.width / 2)) / k.width;
        var dy = (olay.clientY - (k.top + k.height / 2)) / k.height;
        buton.style.transform = "translate(" + (dx * 4).toFixed(2) + "px," + (dy * 3 - 1).toFixed(2) + "px)";
      });
      buton.addEventListener("pointerleave", function () { buton.style.transform = ""; });
    });
  }
  // Sabit sahne: kaydırma ilerlemesi fasıllara ve arka plan katmanlarına
  // dağıtılır. Alt alta kutu yok; tek sahne akar.
  var perde = document.querySelector("[data-perde]");
  if (perde) {
    var fasillar = perde.querySelectorAll("[data-fasil]");
    var kareler = perde.querySelectorAll(".perde-kare");
    var ray = perde.querySelector(".ray-cizgi i");
    var sonSira = -1;
    var cizgile = function () {
      var kutu = perde.getBoundingClientRect();
      var aralik = Math.max(1, perde.offsetHeight - window.innerHeight);
      var p = Math.min(1, Math.max(0, -kutu.top / aralik));
      var sira = Math.min(fasillar.length - 1, Math.floor(p * fasillar.length));
      if (sira !== sonSira) {
        sonSira = sira;
        for (var i = 0; i < fasillar.length; i++) {
          fasillar[i].classList.toggle("etkin", i === sira);
        }
        for (var k = 0; k < kareler.length; k++) {
          kareler[k].classList.toggle("gorunur", k === Math.min(kareler.length - 1, sira));
        }
      }
      var yerel = Math.min(1, Math.max(0, p * fasillar.length - sira));
      perde.style.setProperty("--kay", yerel.toFixed(3));
      if (ray) ray.style.height = Math.max(4, p * 100).toFixed(2) + "%";
    };
    var bekliyor = false;
    var iste = function () {
      if (bekliyor) return;
      bekliyor = true;
      window.requestAnimationFrame(function () {
        bekliyor = false;
        cizgile();
      });
    };
    window.addEventListener("scroll", iste, { passive: true });
    window.addEventListener("resize", iste);
    cizgile();

    // Menü bağlantıları sabit sahnede kendi faslına kaydırır.
    var baglar = document.querySelectorAll('.menuler a[href^="#"], .mobil-menu a[href^="#"]');
    var git = function (e) {
      var hedef = document.getElementById(this.getAttribute("href").slice(1));
      if (!hedef || !perde.contains(hedef)) return;
      e.preventDefault();
      var adim = Number(hedef.getAttribute("data-fasil")) || 0;
      var aralik = perde.offsetHeight - window.innerHeight;
      window.scrollTo({
        top: perde.offsetTop + (adim / fasillar.length) * aralik + 2,
        behavior: azalt ? "auto" : "smooth"
      });
    };
    for (var b = 0; b < baglar.length; b++) baglar[b].addEventListener("click", git);
  }

  // Köz katmanı: yalnız sahne görünürken çalışan hafif parçacık akışı.
  var tuval = document.getElementById("kor");
  if (tuval && !azalt) {
    var cizim = tuval.getContext("2d");
    var taneler = [], canli = false, olcek = 1;
    var boyutla = function () {
      olcek = Math.min(2, window.devicePixelRatio || 1);
      tuval.width = Math.max(1, Math.round(tuval.clientWidth * olcek));
      tuval.height = Math.max(1, Math.round(tuval.clientHeight * olcek));
    };
    var kur = function () {
      taneler = [];
      for (var i = 0; i < 40; i++) {
        taneler.push({
          x: Math.random(),
          y: Math.random(),
          r: .5 + Math.random() * 1.6,
          h: .1 + Math.random() * .38,
          d: .00005 + Math.random() * .00015,
          s: Math.random() * 6.28
        });
      }
    };
    var ciz = function (t) {
      if (!canli) return;
      cizim.clearRect(0, 0, tuval.width, tuval.height);
      for (var i = 0; i < taneler.length; i++) {
        var p = taneler[i];
        p.y -= p.d * 16;
        if (p.y < -.04) { p.y = 1.04; p.x = Math.random(); }
        var x = (p.x + Math.sin(t * .0004 + p.s) * .012) * tuval.width;
        var y = p.y * tuval.height;
        var a = p.h * (.55 + .45 * Math.sin(t * .0011 + p.s));
        cizim.beginPath();
        cizim.arc(x, y, p.r * olcek, 0, 6.2832);
        cizim.fillStyle = "rgba(220,156,78," + a.toFixed(3) + ")";
        cizim.fill();
      }
      window.requestAnimationFrame(ciz);
    };
    boyutla();
    kur();
    window.addEventListener("resize", function () { boyutla(); kur(); });
    if ("IntersectionObserver" in window) {
      new IntersectionObserver(function (kayitlar) {
        var gorunur = kayitlar[0].isIntersecting;
        if (gorunur && !canli) { canli = true; window.requestAnimationFrame(ciz); }
        if (!gorunur) canli = false;
      }, { threshold: 0 }).observe(perde || tuval);
    } else {
      canli = true;
      window.requestAnimationFrame(ciz);
    }
  }

})();
</script>
"##;

/// "Gece" teması: İs ve mangal paleti, pirinç hairline çizgiler, tek ekran.
/// Yalnız `data-tema="gece"` gövdesinde çalışır; klasik düzeni etkilemez.
const GECE_CSS: &str = r#"
body[data-tema=gece]{--f-govde:'Sora','Geist',system-ui,-apple-system,'Segoe UI',sans-serif;--pirinc:#b08d57;--cizgi:color-mix(in srgb,var(--pirinc) 24%,transparent);--dim:color-mix(in srgb,var(--ink) 58%,transparent);--accent-metin:color-mix(in srgb,var(--ink) 86%,var(--pirinc))}
body[data-tema=gece] .serit{background:transparent;color:color-mix(in srgb,var(--pirinc) 80%,var(--ink));border-bottom:1px solid var(--cizgi);font-size:9.5px;letter-spacing:.3em;padding:11px 18px}
body[data-tema=gece] .ust{background:color-mix(in srgb,var(--bg) 88%,transparent);border-bottom:1px solid var(--cizgi)}
body[data-tema=gece] .ust-ic{padding:17px 26px}
body[data-tema=gece] .marka{font-family:var(--f-govde);font-weight:500;font-size:12px;letter-spacing:.22em;text-transform:uppercase}
body[data-tema=gece] .monogram{background:transparent;border:1px solid color-mix(in srgb,var(--pirinc) 45%,transparent);border-radius:0;box-shadow:none;color:color-mix(in srgb,var(--pirinc) 82%,var(--ink));font-family:var(--f-baslik);font-weight:400;font-size:16px}
body[data-tema=gece] .ust-tel{border-radius:0;border-color:var(--cizgi);padding:10px 18px;font-size:10.5px;letter-spacing:.2em;text-transform:uppercase;font-weight:600;color:color-mix(in srgb,var(--ink) 82%,transparent)}
body[data-tema=gece] .ust-tel:hover{background:transparent;border-color:color-mix(in srgb,var(--pirinc) 62%,transparent)}
body[data-tema=gece] .ilerleme{height:1px;background:var(--pirinc);box-shadow:none}
body[data-tema=gece] .hero{padding:clamp(38px,7vh,92px) 0 clamp(38px,6vh,76px)}
body[data-tema=gece] .hero::before{background:color-mix(in srgb,var(--pirinc) 15%,transparent);opacity:.45;filter:blur(90px)}
body[data-tema=gece] .hero::after{background:color-mix(in srgb,var(--accent) 34%,transparent);opacity:.3}
body[data-tema=gece] .hero-ic{grid-template-columns:minmax(0,1fr) minmax(0,.76fr);gap:clamp(26px,4vw,62px);min-height:min(72vh,660px);align-items:end}
body[data-tema=gece] .rozet-cip{border:0;border-radius:0;background:none;padding:0 0 15px;font-size:9.5px;letter-spacing:.28em;color:color-mix(in srgb,var(--pirinc) 85%,var(--ink))}
body[data-tema=gece] .rozet-cip b{width:24px;height:1px;border-radius:0;box-shadow:none;background:var(--pirinc)}
body[data-tema=gece] .manset{font-size:clamp(46px,8.4vw,122px);line-height:.92;letter-spacing:-.028em;font-weight:400}
body[data-tema=gece] .manset-satir:last-child{padding-left:clamp(0px,3.2vw,54px)}
body[data-tema=gece] .hero-alt{margin-top:26px;max-width:44ch;font-size:clamp(14px,1.15vw,16px);line-height:1.78;color:color-mix(in srgb,var(--ink) 66%,transparent)}
body[data-tema=gece] .hero-butonlar{margin-top:34px;gap:6px}
body[data-tema=gece] .buton{border-radius:0;padding:16px 30px;font-size:10.5px;letter-spacing:.22em;text-transform:uppercase;font-weight:600}
body[data-tema=gece] .buton-dolu{background:transparent;border:1px solid color-mix(in srgb,var(--pirinc) 58%,transparent);color:var(--ink);box-shadow:none}
body[data-tema=gece] .buton-dolu:hover{background:color-mix(in srgb,var(--pirinc) 11%,transparent);transform:none;box-shadow:none}
body[data-tema=gece] .buton-cizgi{border:0;padding-left:22px;color:color-mix(in srgb,var(--ink) 60%,transparent)}
body[data-tema=gece] .buton-cizgi:hover{background:none;color:var(--ink)}
body[data-tema=gece] .durum,body[data-tema=gece] .rozet,body[data-tema=gece] .akis,body[data-tema=gece] .istatistik{display:none}
body[data-tema=gece] .sahne{aspect-ratio:4/5;border:0;border-radius:0;max-height:min(72vh,660px)}
body[data-tema=gece] .sahne-foto{filter:grayscale(.34) contrast(1.05) brightness(.82) saturate(.85)}
body[data-tema=gece] .sahne-alt{left:18px;right:18px;bottom:15px;font-size:8.5px;letter-spacing:.32em;color:color-mix(in srgb,var(--ink) 52%,transparent)}
body[data-tema=gece] .monogram-buyuk{-webkit-text-stroke:1px color-mix(in srgb,var(--pirinc) 38%,transparent);opacity:.38}
body[data-tema=gece] .satici{border-top:1px solid var(--cizgi);background:transparent}
body[data-tema=gece] .satici h2{font-weight:400;font-size:clamp(22px,2.6vw,32px);line-height:1.12;letter-spacing:-.01em;max-width:26ch}
body[data-tema=gece] .satici .buton{border-radius:0;border:1px solid color-mix(in srgb,var(--pirinc) 52%,transparent);color:var(--ink)}
body[data-tema=gece] .altbilgi{border-top:1px solid var(--cizgi);background:transparent}
body[data-tema=gece] .alt-not{max-width:64ch;font-size:11px;line-height:1.75;color:color-mix(in srgb,var(--ink) 44%,transparent)}
body[data-tema=gece] .alt-baglar a{font-size:9.5px;letter-spacing:.22em;text-transform:uppercase;color:color-mix(in srgb,var(--pirinc) 76%,var(--ink))}
body[data-tema=gece] .mobil-cta{border-radius:0;background:transparent;border:1px solid color-mix(in srgb,var(--pirinc) 52%,transparent);color:var(--ink);font-size:10.5px;letter-spacing:.22em;text-transform:uppercase;box-shadow:none}
body[data-tema=gece] #fener{background:radial-gradient(420px circle at var(--fx,50%) var(--fy,50%),color-mix(in srgb,var(--pirinc) 14%,transparent),transparent 70%)}
body[data-tema=gece] .bolum{border-top:1px solid var(--cizgi);padding:clamp(52px,7.6vw,104px) 0}
body[data-tema=gece] .bolum-ust{margin-bottom:clamp(26px,3.6vw,48px);padding-top:0}
body[data-tema=gece] .bolum-ust::before{display:none}
body[data-tema=gece] .bolum-ust .no{font-family:var(--f-govde);font-size:9.5px;letter-spacing:.28em;color:color-mix(in srgb,var(--pirinc) 85%,var(--ink))}
body[data-tema=gece] .bolum-ust .ince{font-size:9.5px;letter-spacing:.28em;text-transform:uppercase;color:var(--dim)}
body[data-tema=gece] .gece-baslik{font-family:var(--f-baslik);font-weight:400;font-size:clamp(30px,4.4vw,58px);line-height:1.02;letter-spacing:-.022em;max-width:18ch;margin:0}
body[data-tema=gece] .gece-not{margin:18px 0 0;max-width:52ch;font-size:clamp(13.5px,1.1vw,15px);line-height:1.8;color:color-mix(in srgb,var(--ink) 58%,transparent)}
body[data-tema=gece] .hikaye h2{font-family:var(--f-baslik);font-weight:400;font-size:clamp(28px,3.6vw,46px);line-height:1.06;letter-spacing:-.02em;margin:0}
body[data-tema=gece] .hikaye-buyuk{font-family:var(--f-baslik);font-size:clamp(20px,2.4vw,30px);line-height:1.34;letter-spacing:-.012em;color:var(--ink)}
body[data-tema=gece] .hikaye p:not(.hikaye-buyuk){color:color-mix(in srgb,var(--ink) 55%,transparent);font-size:14.5px;line-height:1.85;max-width:54ch}
body[data-tema=gece] .hikaye-gorsel{border:0;border-radius:0;aspect-ratio:5/4;max-height:460px;filter:grayscale(.3) contrast(1.04) brightness(.86)}
body[data-tema=gece] .gece-liste{margin-top:clamp(26px,3.4vw,44px);border-top:1px solid var(--cizgi)}
body[data-tema=gece] .gece-liste .satir{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1.05fr);gap:clamp(14px,4vw,48px);align-items:baseline;border:0;border-bottom:1px solid var(--cizgi);border-radius:0;background:none;box-shadow:none;padding:22px 0}
body[data-tema=gece] .gece-liste .satir:hover{background:none;transform:none}
body[data-tema=gece] .gece-liste .satir h3{font-family:var(--f-baslik);font-weight:400;font-size:clamp(20px,2.3vw,28px);line-height:1.14;letter-spacing:-.012em;margin:0}
body[data-tema=gece] .gece-liste .satir p{margin:0;font-size:13.5px;line-height:1.75;color:color-mix(in srgb,var(--ink) 54%,transparent);max-width:46ch}
body[data-tema=gece] .gece-iletisim{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1.05fr);gap:clamp(30px,5vw,72px);align-items:start}
body[data-tema=gece] .gece-bilgi{margin-top:clamp(24px,3vw,38px);border-top:1px solid var(--cizgi)}
body[data-tema=gece] .saatler{border:0;border-radius:0;background:none;box-shadow:none;padding:0}
body[data-tema=gece] .saat-satir{border:0;border-bottom:1px solid var(--cizgi);padding:11px 0;font-size:12px;letter-spacing:.06em;color:color-mix(in srgb,var(--ink) 62%,transparent)}
body[data-tema=gece] .saat-satir b{font-family:var(--f-govde);font-weight:500;font-size:11.5px;letter-spacing:.12em;color:color-mix(in srgb,var(--pirinc) 80%,var(--ink))}
body[data-tema=gece] .ilet-kart{border:0;border-bottom:1px solid var(--cizgi);border-radius:0;background:none;box-shadow:none;padding:15px 0}
body[data-tema=gece] .ilet-kart:hover{background:none;transform:none}
body[data-tema=gece] .ilet-kart small{font-size:9px;letter-spacing:.26em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 46%,transparent)}
body[data-tema=gece] .ilet-kart b{font-family:var(--f-baslik);font-weight:400;font-size:17px;letter-spacing:-.01em}
body[data-tema=gece] .gece-form{border-top:1px solid var(--cizgi)}
body[data-tema=gece] .gece-form .alan{display:grid;gap:7px;padding:15px 0;border-bottom:1px solid var(--cizgi);transition:border-color .4s}
body[data-tema=gece] .gece-form label{font-size:9px;letter-spacing:.26em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 52%,transparent)}
body[data-tema=gece] .gece-form input,body[data-tema=gece] .gece-form textarea{border:0;border-radius:0;background:transparent;padding:0;width:100%;resize:none;color:var(--ink);font-family:var(--f-baslik);font-size:clamp(17px,1.7vw,21px);letter-spacing:-.01em;outline:none}
body[data-tema=gece] .gece-form input:focus,body[data-tema=gece] .gece-form textarea:focus{outline:none;box-shadow:none}
body[data-tema=gece] .gece-form .alan:focus-within{border-bottom-color:color-mix(in srgb,var(--pirinc) 64%,transparent)}
body[data-tema=gece] .gece-form .buton{width:100%;justify-content:center;margin-top:24px}
body[data-tema=gece] .form-not{min-height:16px;margin:13px 0 0;font-size:11px;line-height:1.6;letter-spacing:.04em;color:color-mix(in srgb,var(--pirinc) 82%,var(--ink))}
body[data-tema=gece] .menuler{gap:clamp(14px,2.4vw,30px)}
body[data-tema=gece] .menuler a{font-size:9.5px;letter-spacing:.24em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 58%,transparent)}
body[data-tema=gece] .menuler a:hover{color:var(--ink)}
body[data-tema=gece] .mobil-dugme{border:1px solid var(--cizgi);border-radius:0;background:none;color:var(--ink)}
body[data-tema=gece] .mobil-menu{border-top:1px solid var(--cizgi);background:color-mix(in srgb,var(--bg) 96%,transparent)}
body[data-tema=gece] .mobil-menu a{font-size:10px;letter-spacing:.24em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 68%,transparent);border-bottom:1px solid var(--cizgi)}
body[data-tema=gece] .gece-harita{margin-top:clamp(22px,3vw,40px);padding-top:clamp(16px,2vw,24px);border-top:1px solid var(--cizgi)}
body[data-tema=gece] .gece-harita .harita{border:0;border-radius:0;background:none;box-shadow:none}
body[data-tema=gece] .gece-harita iframe{height:clamp(240px,34vw,400px);filter:grayscale(.6) contrast(1.04) brightness(.76)}
body[data-tema=gece] .gece-harita .harita-alt{border-top:1px solid var(--cizgi);background:none;font-size:9.5px;letter-spacing:.22em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 48%,transparent)}
body[data-tema=gece] .gece-harita .harita-alt a{color:color-mix(in srgb,var(--pirinc) 80%,var(--ink));font-weight:500;letter-spacing:.22em}
body[data-tema=gece] .perde{position:relative;height:400svh;background:var(--bg)}
body[data-tema=gece] .perde-sabit{position:sticky;top:0;height:100svh;overflow:hidden;display:flex;flex-direction:column}
body[data-tema=gece] .perde-arka{position:absolute;inset:0;z-index:0;background:radial-gradient(120% 90% at 70% 92%,color-mix(in srgb,var(--accent) 24%,#120a06),#0a0806 62%)}
body[data-tema=gece] .perde-kare{position:absolute;inset:0;opacity:0;transform:scale(1.07);transition:opacity 1.15s ease,transform 1.8s cubic-bezier(.19,.78,.18,1)}
body[data-tema=gece] .perde-kare.gorunur{opacity:1;transform:scale(1)}
body[data-tema=gece] .perde-kare img{width:100%;height:100%;object-fit:cover;object-position:50% 55%;filter:grayscale(.34) contrast(1.06) brightness(.44) saturate(.9)}
body[data-tema=gece] .perde-kare.doku{background:radial-gradient(90% 70% at 62% 42%,color-mix(in srgb,var(--accent) 38%,#1a0f08),#0b0907 72%)}
body[data-tema=gece] .perde-perde{position:absolute;inset:0;z-index:2;background:linear-gradient(90deg,rgba(10,8,7,.93) 0%,rgba(10,8,7,.58) 44%,rgba(10,8,7,.26) 100%),linear-gradient(180deg,rgba(10,8,7,.72),transparent 32%,rgba(10,8,7,.55) 84%,var(--bg) 100%)}
body[data-tema=gece] .sinema-kor{position:absolute;inset:0;z-index:1;pointer-events:none}
body[data-tema=gece] #kor{display:block;width:100%;height:100%;opacity:.8;mix-blend-mode:screen}
body[data-tema=gece] .perde-ic{position:relative;z-index:3;flex:1;min-height:0;display:block;width:min(1240px,92vw);max-width:1240px;margin:0 auto}
body[data-tema=gece] .fasil{position:absolute;inset:0;display:grid;grid-template-columns:minmax(0,1.1fr) minmax(340px,.9fr);align-items:center;gap:clamp(28px,4vw,64px);box-sizing:border-box;padding:clamp(86px,12vh,124px) 0 clamp(22px,4vh,46px);opacity:0;transform:translateY(30px);pointer-events:none;transition:opacity .8s ease,transform 1s cubic-bezier(.19,.78,.18,1)}
body[data-tema=gece] .fasil.etkin{opacity:1;transform:none;pointer-events:auto}
body[data-tema=gece] .fasil-sol{max-width:100%;min-width:0}
body[data-tema=gece] .fasil-no{display:block;margin-bottom:clamp(12px,2vh,22px);font-size:9px;letter-spacing:.3em;text-transform:uppercase;color:color-mix(in srgb,var(--pirinc) 80%,var(--ink))}
body[data-tema=gece] .fasil-sag{justify-self:end;width:100%;max-width:min(520px,44ch);max-height:calc(100svh - 200px);overflow:auto;scrollbar-width:none;padding:clamp(18px,2.2vw,30px);background:rgba(0,0,0,.6);border:1px solid color-mix(in srgb,var(--pirinc) 22%,transparent);backdrop-filter:blur(4px)}
body[data-tema=gece] .fasil .gece-baslik{max-width:none;font-size:clamp(28px,3.4vw,46px);line-height:1.04}
body[data-tema=gece] .perde .manset{max-width:19ch;margin:0;font-size:clamp(40px,5.4vw,86px);line-height:.98;text-shadow:0 2px 30px rgba(0,0,0,.55)}
body[data-tema=gece] .perde .hero-alt{margin-top:20px;max-width:44ch;text-shadow:0 1px 16px rgba(0,0,0,.5)}
body[data-tema=gece] .perde .hero-butonlar{margin-top:clamp(18px,3vh,28px)}
body[data-tema=gece] .perde .hikaye{display:block}
body[data-tema=gece] .perde .hikaye h2{margin:0 0 12px;font-family:var(--f-baslik);font-weight:400;font-size:clamp(21px,2.1vw,29px);line-height:1.1;letter-spacing:-.015em}
body[data-tema=gece] .perde .hikaye p{margin:0 0 12px;font-size:13px;line-height:1.8;color:color-mix(in srgb,var(--ink) 58%,transparent)}
body[data-tema=gece] .perde .hikaye-buyuk{font-family:var(--f-baslik);font-size:clamp(17px,1.8vw,22px);line-height:1.36;letter-spacing:-.01em;color:color-mix(in srgb,var(--ink) 88%,transparent)}
body[data-tema=gece] .perde .gece-liste{margin:0;border-top:1px solid var(--cizgi)}
body[data-tema=gece] .perde .gece-liste .satir{display:grid;grid-template-columns:1fr;gap:3px;padding:12px 0;border:0;border-bottom:1px solid var(--cizgi)}
body[data-tema=gece] .perde .gece-liste .satir h3{font-size:clamp(16px,1.5vw,20px);margin:0}
body[data-tema=gece] .perde .gece-liste .satir p{font-size:11.5px;line-height:1.65;max-width:none}
body[data-tema=gece] .perde .gece-bilgi{margin-top:16px}
body[data-tema=gece] .perde .saat-satir{padding:8px 0;font-size:10.5px}
body[data-tema=gece] .perde .saat-satir b{font-size:10.5px}
body[data-tema=gece] .perde .ilet-kart{padding:10px 0}
body[data-tema=gece] .perde .ilet-kart b{font-size:15px}
body[data-tema=gece] .perde .gece-harita{margin-top:14px;padding-top:12px;border-top:1px solid var(--cizgi)}
body[data-tema=gece] .perde .gece-harita iframe{height:clamp(170px,26vh,250px)}
body[data-tema=gece] .perde .gece-form .alan{padding:10px 0}
body[data-tema=gece] .perde .gece-form input,body[data-tema=gece] .perde .gece-form textarea{font-size:clamp(15px,1.3vw,18px)}
body[data-tema=gece] .perde .gece-form .buton{margin-top:16px}
body[data-tema=gece] .perde-ray{position:absolute;right:clamp(12px,2.2vw,26px);top:50%;z-index:4;display:flex;flex-direction:column;align-items:center;gap:10px;transform:translateY(-50%);font-size:8.5px;letter-spacing:.24em;color:color-mix(in srgb,var(--ink) 40%,transparent)}
body[data-tema=gece] .ray-cizgi{position:relative;display:block;width:1px;height:clamp(64px,13vh,116px);background:color-mix(in srgb,var(--ink) 16%,transparent)}
body[data-tema=gece] .ray-cizgi i{position:absolute;left:-1px;top:0;display:block;width:3px;height:4%;background:var(--pirinc);transition:height .35s ease}
body[data-tema=gece] .manset{font-family:'Cormorant Garamond','Instrument Serif',Georgia,serif;font-weight:400;font-size:clamp(44px,6vw,96px);line-height:.98;letter-spacing:-.012em}
body[data-tema=gece] .gece-baslik{font-family:'Cormorant Garamond','Instrument Serif',Georgia,serif;font-weight:400;font-size:clamp(32px,3.8vw,52px);line-height:1.02;letter-spacing:-.008em;max-width:16ch}
body[data-tema=gece] .hikaye h2{font-family:'Cormorant Garamond','Instrument Serif',Georgia,serif;font-weight:400;font-size:clamp(22px,2.2vw,30px);line-height:1.12}
body[data-tema=gece] .fasil-no,body[data-tema=gece] .ince,body[data-tema=gece] .rozet-cip{letter-spacing:.34em}
body[data-tema=gece] .perde .hero-alt,body[data-tema=gece] .gece-not,body[data-tema=gece] .perde .hikaye p,body[data-tema=gece] .perde .gece-liste .satir p{letter-spacing:.012em}
body[data-tema=gece] .perde .buton{letter-spacing:.26em}
body[data-tema=gece] .klm{display:inline-block;overflow:hidden;vertical-align:top}
body[data-tema=gece] .klm i{display:inline-block;font-style:inherit;transform:translateY(112%);opacity:0;filter:blur(9px);transition:transform 1.1s cubic-bezier(.19,.78,.18,1),opacity .9s ease,filter 1s ease;transition-delay:calc(var(--i,0) * 64ms)}
body[data-tema=gece] .fasil.etkin .klm i{transform:none;opacity:1;filter:none}
body[data-tema=gece] .fasil.etkin .gece-baslik{transform:translateY(calc(var(--kay,0) * -24px))}
body[data-tema=gece] .fasil.etkin .fasil-sag{transform:translateY(calc(var(--kay,0) * 16px))}
body[data-tema=gece] .perde-sabit::after{content:"";position:absolute;inset:0;z-index:2;pointer-events:none;opacity:.055;mix-blend-mode:overlay;background-image:url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='180' height='180'><filter id='g'><feTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='3' stitchTiles='stitch'/></filter><rect width='180' height='180' filter='url(%23g)'/></svg>")}
body[data-tema=gece] .perde-sabit::before{content:"";position:absolute;left:-10%;right:-10%;bottom:-18%;height:62%;z-index:1;pointer-events:none;filter:blur(46px);background:radial-gradient(62% 60% at 50% 100%,color-mix(in srgb,var(--accent) 30%,transparent),transparent 72%);animation:korNefes 9s ease-in-out infinite}
@keyframes korNefes{0%,100%{opacity:.38;transform:translateY(8px) scale(1)}50%{opacity:.62;transform:translateY(-6px) scale(1.03)}}
body[data-tema=gece] .perde .buton{position:relative;overflow:hidden;transition:transform .35s cubic-bezier(.2,.8,.2,1),background .4s,border-color .4s;will-change:transform}
body[data-tema=gece] .perde .buton::after{content:"";position:absolute;inset:-1px;padding:1px;pointer-events:none;opacity:0;transition:opacity .4s;background:conic-gradient(from 0deg,transparent 0deg,transparent 246deg,color-mix(in srgb,var(--pirinc) 92%,transparent) 318deg,transparent 360deg);-webkit-mask:linear-gradient(#000 0 0) content-box,linear-gradient(#000 0 0);mask:linear-gradient(#000 0 0) content-box,linear-gradient(#000 0 0);-webkit-mask-composite:xor;mask-composite:exclude}
body[data-tema=gece] .perde .buton:hover::after{opacity:1;animation:pirincDon 3s linear infinite}
@keyframes pirincDon{from{transform:rotate(0deg)}to{transform:rotate(360deg)}}
@media (max-width:1000px){
  body[data-tema=gece] .hero-ic{grid-template-columns:1fr;gap:24px;min-height:0;align-items:start}
  body[data-tema=gece] .manset-satir:last-child{padding-left:0}
  body[data-tema=gece] .sahne{aspect-ratio:3/4;max-height:60vh}
  body[data-tema=gece] .hero-butonlar{gap:8px}
  body[data-tema=gece] .buton{flex:1 1 100%;justify-content:center;padding:15px 22px}
  body[data-tema=gece] .hero-alt{max-width:none}
  body[data-tema=gece] .bolum{padding:clamp(34px,7vw,58px) 0}
  body[data-tema=gece] .gece-iletisim{grid-template-columns:1fr;gap:34px}
  body[data-tema=gece] .gece-liste .satir{grid-template-columns:1fr;gap:7px;padding:18px 0}
  body[data-tema=gece] .menuler{display:none}
  body[data-tema=gece] .mobil-dugme{display:flex;align-items:center;justify-content:center}
  body[data-tema=gece] .gece-harita iframe{height:250px}
  body[data-tema=gece] .fasil{grid-template-columns:1fr;align-content:center;gap:18px;padding:clamp(80px,12vh,110px) 0 26px}
  body[data-tema=gece] .fasil-sag{max-height:none;overflow:visible;max-width:100%;padding:16px}
  body[data-tema=gece] .fasil-sol{max-width:none}
  body[data-tema=gece] .fasil-sag{max-width:none;justify-self:start}
  body[data-tema=gece] .perde .manset{max-width:none;font-size:clamp(34px,9.6vw,54px)}
  body[data-tema=gece] .gece-baslik{font-size:clamp(26px,7.4vw,38px);max-width:none}
  body[data-tema=gece] .perde-sabit::before{filter:blur(30px)}
  body[data-tema=gece] .perde .hikaye p{font-size:12.5px;margin-bottom:8px}
  body[data-tema=gece] .perde-ray{display:none}
  body[data-tema=gece] .perde .gece-harita iframe{height:150px}
  body[data-tema=gece] .perde .gece-liste .satir{padding:9px 0}
  body[data-tema=gece] .akis-kart{width:41vw}
}
"#;
/// "Işık" yerleşimi: aynı sahne mimarisi, açık lüks palet. Fildişi zemin,
/// espresso metin, pirinç çizgi; hiçbir yüzey koyu değil.
const ISIK_CSS: &str = r#"
body[data-tema=isik]{--f-govde:'Plus Jakarta Sans','Geist',system-ui,-apple-system,'Segoe UI',sans-serif;--pirinc:#b8912f;--bg:#fcfbf9;--ink:#2c221e;--cizgi:color-mix(in srgb,var(--ink) 15%,transparent);--dim:color-mix(in srgb,var(--ink) 55%,transparent);--accent-metin:color-mix(in srgb,var(--ink) 88%,var(--pirinc))}
body[data-tema=isik] .serit{background:#f4f1ea;color:color-mix(in srgb,var(--ink) 60%,transparent);border-bottom:1px solid var(--cizgi)}
body[data-tema=isik] .ust{background:color-mix(in srgb,#fcfbf9 90%,transparent);border-bottom:1px solid var(--cizgi)}
body[data-tema=isik] .monogram{border-color:color-mix(in srgb,var(--pirinc) 55%,transparent);color:var(--ink);background:transparent}
body[data-tema=isik] .ust-tel{border-color:var(--cizgi);color:var(--ink)}
body[data-tema=isik] .perde{background:var(--bg)}
body[data-tema=isik] .perde-arka{background:radial-gradient(120% 90% at 70% 92%,#f5f1e8,#fcfbf9 64%)}
body[data-tema=isik] .perde-kare img{filter:saturate(1.02) contrast(1.03) brightness(1.1)}
body[data-tema=isik] .perde-kare.doku{background:radial-gradient(90% 70% at 62% 42%,#efe7d7,#fbf8f2 74%)}
body[data-tema=isik] .perde-perde{background:linear-gradient(90deg,rgba(252,251,249,.97) 0%,rgba(252,251,249,.9) 44%,rgba(252,251,249,.46) 100%),linear-gradient(180deg,rgba(252,251,249,.6),transparent 30%,rgba(252,251,249,.55) 84%,var(--bg) 100%)}
body[data-tema=isik] .perde-sabit::after{opacity:.028;mix-blend-mode:multiply}
body[data-tema=isik] .perde-sabit::before{background:radial-gradient(62% 60% at 50% 100%,rgba(212,175,55,.18),transparent 72%);filter:blur(52px);animation-duration:12s}
body[data-tema=isik] #kor{opacity:.2;mix-blend-mode:multiply}
body[data-tema=isik] #fener{background:radial-gradient(560px circle at var(--fx,50%) var(--fy,50%),rgba(212,175,55,.22),transparent 70%);mix-blend-mode:multiply}
@media (hover:hover) and (pointer:fine){html.tema-acik body[data-tema=isik] #fener.acik{opacity:1}}
body[data-tema=isik] .fasil-sag{background:rgba(255,255,255,.74);border:1px solid rgba(44,34,30,.08);box-shadow:0 24px 70px -40px rgba(44,34,30,.45);backdrop-filter:blur(7px)}
body[data-tema=isik] .fasil-no,body[data-tema=isik] .ince,body[data-tema=isik] .rozet-cip{color:color-mix(in srgb,var(--pirinc) 70%,var(--ink))}
body[data-tema=isik] .manset,body[data-tema=isik] .gece-baslik,body[data-tema=isik] .hikaye h2{color:#2c221e;letter-spacing:.012em;text-shadow:none}
body[data-tema=isik] .perde .hero-alt,body[data-tema=isik] .gece-not,body[data-tema=isik] .perde .hikaye p{color:color-mix(in srgb,var(--ink) 66%,transparent);text-shadow:none}
body[data-tema=isik] .buton{border-radius:0}
body[data-tema=isik] .buton-dolu{background:var(--ink);border:1px solid var(--ink);color:#fcfbf9}
body[data-tema=isik] .buton-dolu:hover{background:#3b2d26;border-color:#3b2d26}
body[data-tema=isik] .buton-cizgi{color:color-mix(in srgb,var(--ink) 70%,transparent)}
body[data-tema=isik] .perde .buton::after{background:conic-gradient(from 0deg,transparent 0deg,transparent 246deg,rgba(212,175,55,.95) 318deg,transparent 360deg)}
body[data-tema=isik] .perde .gece-liste{display:grid;gap:10px;border-top:0}
body[data-tema=isik] .perde .gece-liste .satir{background:#fff;border:1px solid rgba(44,34,30,.08);border-radius:2px;padding:16px 18px;transition:transform .5s cubic-bezier(.2,.8,.2,1),box-shadow .5s ease,border-color .4s ease}
body[data-tema=isik] .perde .gece-liste .satir:hover{transform:translateY(-3px);box-shadow:0 22px 44px -30px rgba(44,34,30,.5);border-color:color-mix(in srgb,var(--pirinc) 45%,transparent)}
body[data-tema=isik] .perde .gece-liste .satir p{color:color-mix(in srgb,var(--ink) 58%,transparent)}
body[data-tema=isik] .perde .saat-satir{border-bottom-color:var(--cizgi);color:color-mix(in srgb,var(--ink) 62%,transparent)}
body[data-tema=isik] .perde .saat-satir b{color:color-mix(in srgb,var(--pirinc) 62%,var(--ink))}
body[data-tema=isik] .perde .gece-harita{border-top-color:var(--cizgi)}
body[data-tema=isik] .gece-harita iframe{filter:grayscale(.3) contrast(1.02) brightness(1.05)}
body[data-tema=isik] .gece-harita .harita-alt{border-top-color:var(--cizgi);color:color-mix(in srgb,var(--ink) 50%,transparent)}
body[data-tema=isik] .gece-harita .harita-alt a{color:var(--ink)}
body[data-tema=isik] .gece-form input,body[data-tema=isik] .gece-form textarea{color:var(--ink)}
body[data-tema=isik] .form-not{color:color-mix(in srgb,var(--pirinc) 70%,var(--ink))}
body[data-tema=isik] .perde-ray{color:color-mix(in srgb,var(--ink) 36%,transparent)}
body[data-tema=isik] .ray-cizgi{background:color-mix(in srgb,var(--ink) 12%,transparent)}
body[data-tema=isik] .satici{border-top:1px solid var(--cizgi);background:#f7f3ea}
body[data-tema=isik] .satici h2{color:var(--ink)}
body[data-tema=isik] .altbilgi{border-top:1px solid var(--cizgi);background:var(--bg)}
body[data-tema=isik] .mobil-cta{background:var(--ink);color:#fcfbf9;border-color:var(--ink)}
body[data-tema=isik] .mobil-dugme{border-color:var(--cizgi);color:var(--ink)}
body[data-tema=isik] .mobil-menu{border-top:1px solid var(--cizgi);background:#fcfbf9}
"#;

/// Acil sektör katmanı: Ken Burns, cam rozetler, konum butonu, mobil bar,
/// yorum kaydırıcısı ve yol yardım asistanı. Yalnız bu katman eklenir; klasik
/// düzen ve "gece"/"ışık" temaları olduğu gibi kalır.
const EK_CSS: &str = r#"
/* --- Ken Burns: görseller yavaşça yakınlaşır, parallax ile birlikte --- */
@keyframes kenburns{0%{transform:scale(1.02) translate3d(0,0,0)}55%{transform:scale(1.1) translate3d(-1.2%,-1%,0)}100%{transform:scale(1.16) translate3d(1.4%,-1.8%,0)}}
.sahne.foto .sahne-foto{animation:kenburns 24s ease-in-out infinite alternate;will-change:transform}
.galeri-kare img{animation:kenburns 28s ease-in-out infinite alternate;will-change:transform}
.hikaye-gorsel img{animation:kenburns 30s ease-in-out infinite alternate;will-change:transform}
.perde-kare img{animation:kenburns 34s ease-in-out infinite alternate;will-change:transform}
.sahne.foto{transform:translateY(calc(var(--kaydir,0px) * -1))}
.galeri-kare{transform:translateY(calc(var(--kaydir,0px) * .35))}
@media (prefers-reduced-motion:reduce){
  .sahne.foto .sahne-foto,.galeri-kare img,.hikaye-gorsel img,.perde-kare img{animation:none}
  .sahne.foto,.galeri-kare{transform:none}
}
/* --- cam rozetler --- */
.hero-rozetler{display:flex;flex-wrap:wrap;gap:8px;margin-top:18px}
.hcip{display:inline-flex;align-items:center;gap:8px;padding:8px 13px;border-radius:999px;font-size:11.5px;font-weight:600;letter-spacing:.03em;color:color-mix(in srgb,var(--ink) 84%,transparent);background:color-mix(in srgb,var(--bg) 58%,transparent);border:1px solid color-mix(in srgb,var(--accent) 26%,transparent);-webkit-backdrop-filter:blur(14px) saturate(1.15);backdrop-filter:blur(14px) saturate(1.15);box-shadow:0 10px 26px -20px color-mix(in srgb,var(--ink) 60%,transparent)}
.hcip.canli b{width:7px;height:7px;border-radius:50%;background:#4ade80;box-shadow:0 0 0 0 rgba(74,222,128,.55);animation:nabiz 2.2s infinite}
.hcip.canli{color:color-mix(in srgb,#2f9e63 78%,var(--ink));border-color:color-mix(in srgb,#4ade80 42%,transparent)}
/* --- canlı konum butonu --- */
.buton-konum{position:relative;overflow:hidden;background:linear-gradient(120deg,var(--accent),color-mix(in srgb,var(--accent) 55%,#12b3a6));color:var(--accent-karsi);box-shadow:0 18px 44px -18px var(--accent)}
.buton-konum::after{content:"";position:absolute;inset:0;background:linear-gradient(110deg,transparent 20%,rgba(255,255,255,.38) 42%,transparent 62%);transform:translateX(-120%);animation:parilti 4.2s ease-in-out infinite}
@keyframes parilti{0%,55%{transform:translateX(-120%)}85%{transform:translateX(120%)}100%{transform:translateX(120%)}}
.buton-konum svg{width:16px;height:16px}
/* --- sabit mobil acil barı --- */
.mobil-bar{display:none;position:fixed;left:10px;right:10px;bottom:10px;z-index:80;gap:8px;padding:8px;border-radius:16px;background:color-mix(in srgb,var(--bg) 72%,transparent);border:1px solid var(--cizgi);-webkit-backdrop-filter:blur(18px) saturate(1.2);backdrop-filter:blur(18px) saturate(1.2);box-shadow:0 20px 50px -20px rgba(0,0,0,.75)}
.mobil-bar a{flex:1;display:flex;align-items:center;justify-content:center;gap:8px;padding:13px 10px;border-radius:11px;font-weight:750;font-size:13px;letter-spacing:.01em}
.mobil-bar svg{width:16px;height:16px}
.mobil-ara{background:color-mix(in srgb,var(--ink) 8%,transparent);color:var(--ink);border:1px solid var(--cizgi)}
.mobil-konum{background:linear-gradient(120deg,var(--accent),color-mix(in srgb,var(--accent) 58%,#12b3a6));color:var(--accent-karsi);box-shadow:0 12px 30px -16px var(--accent)}
@media (max-width:1000px){
  .mobil-bar{display:flex}
  .mobil-cta{display:none}
  body.acil-bar{padding-bottom:96px}
}
/* --- yorum kaydırıcısı --- */
.yorum-kaydirici{position:relative;margin-top:4px}
.yorum-iz{display:flex;gap:18px;overflow-x:auto;scroll-snap-type:x mandatory;scrollbar-width:none;padding:2px 2px 6px}
.yorum-iz::-webkit-scrollbar{display:none}
.yorum-iz .alinti{flex:0 0 clamp(272px,46%,520px);scroll-snap-align:start}
.yorum-kontroller{display:flex;align-items:center;gap:9px;margin-top:14px}
.yk-btn{width:34px;height:34px;border-radius:50%;border:1px solid var(--cizgi);background:var(--kart);color:var(--ink);font-size:18px;line-height:1;cursor:pointer;transition:border-color .3s,background .3s}
.yk-btn:hover{border-color:color-mix(in srgb,var(--accent) 55%,transparent);background:color-mix(in srgb,var(--accent) 10%,transparent)}
.yorum-noktalar{display:flex;align-items:center;gap:6px;margin-left:auto}
.yorum-noktalar button{width:7px;height:7px;border-radius:50%;border:0;padding:0;background:color-mix(in srgb,var(--ink) 22%,transparent);cursor:pointer;transition:width .3s,background .3s}
.yorum-noktalar button.etkin{width:20px;border-radius:99px;background:var(--accent)}
@media (max-width:1000px){.yorum-iz .alinti{flex-basis:86%}}
/* --- yol yardım asistanı --- */
.asistan{position:fixed;right:18px;bottom:18px;z-index:90;display:flex;flex-direction:column;align-items:flex-end;gap:10px}
.asistan-dugme{display:inline-flex;align-items:center;gap:9px;padding:12px 16px;border-radius:999px;border:1px solid color-mix(in srgb,var(--accent) 34%,transparent);background:color-mix(in srgb,var(--bg) 74%,transparent);color:var(--ink);font-weight:700;font-size:12.5px;cursor:pointer;-webkit-backdrop-filter:blur(16px);backdrop-filter:blur(16px);box-shadow:0 18px 40px -22px rgba(0,0,0,.8);transition:transform .3s,border-color .3s}
.asistan-dugme:hover{transform:translateY(-2px);border-color:var(--accent)}
.asistan-nokta{width:8px;height:8px;border-radius:50%;background:#4ade80;box-shadow:0 0 0 0 rgba(74,222,128,.6);animation:nabiz 2.4s infinite}
.asistan-panel{width:min(344px,calc(100vw - 28px));padding:16px;border-radius:18px;background:color-mix(in srgb,var(--bg) 92%,transparent);border:1px solid var(--cizgi);-webkit-backdrop-filter:blur(20px) saturate(1.2);backdrop-filter:blur(20px) saturate(1.2);box-shadow:0 30px 70px -30px rgba(0,0,0,.85);animation:asistanAc .28s ease both}
@keyframes asistanAc{from{opacity:0;transform:translateY(12px) scale(.98)}to{opacity:1;transform:none}}
.asistan-panel[hidden]{display:none}
.asistan-ust{display:flex;align-items:center;justify-content:space-between;gap:10px}
.asistan-ust b{font-family:var(--f-baslik);font-size:15px;letter-spacing:-.01em}
.asistan-kapat{width:26px;height:26px;border-radius:50%;border:1px solid var(--cizgi);background:transparent;color:var(--dim);font-size:15px;line-height:1;cursor:pointer}
.asistan-soru{margin:8px 0 12px;font-size:12.5px;line-height:1.55;color:var(--dim)}
.asistan-secim{display:grid;grid-template-columns:1fr 1fr;gap:8px}
.asistan-secim button{padding:11px 8px;border-radius:11px;border:1px solid var(--cizgi);background:var(--kart);color:var(--ink);font-weight:650;font-size:12.5px;cursor:pointer;transition:border-color .25s,background .25s,transform .25s}
.asistan-secim button:hover{border-color:color-mix(in srgb,var(--accent) 52%,transparent);transform:translateY(-1px)}
.asistan-secim button.secili{background:color-mix(in srgb,var(--accent) 16%,transparent);border-color:var(--accent);color:color-mix(in srgb,var(--accent) 72%,var(--ink))}
.asistan-alan{display:block;margin-top:10px}
.asistan-alan span{display:block;font-size:10.5px;letter-spacing:.08em;text-transform:uppercase;color:var(--dim);margin-bottom:5px}
.asistan-alan input{width:100%;padding:10px 12px;border-radius:10px;border:1px solid var(--cizgi);background:color-mix(in srgb,var(--bg) 80%,transparent);color:var(--ink);font:inherit;font-size:12.5px;outline:none}
.asistan-alan input:focus{border-color:var(--accent)}
.asistan-gonder{width:100%;margin-top:11px;padding:13px;border-radius:11px;border:0;background:var(--accent);color:var(--accent-karsi);font-weight:750;font-size:13px;cursor:pointer}
.asistan-gonder:hover{filter:brightness(1.05)}
.asistan-not{min-height:14px;margin:9px 0 0;font-size:11px;line-height:1.5;color:var(--dim)}
@media (max-width:1000px){.asistan{right:12px;bottom:96px}.asistan-dugme{padding:11px 14px;font-size:12px}}
/* --- hareket kimliği: sektörün tempolu ya da sakin ritmi --- */
body[data-hareket=canli] .akis-ic{animation-duration:26s}
body[data-hareket=canli] .hcip.canli b{animation-duration:1.6s}
body[data-hareket=sakin] .akis-ic{animation-duration:52s}
/* --- sinematik kaydırma --- */
.sinema{padding:clamp(18px,3vw,34px) 0 clamp(6px,1.6vw,18px)}
.sinema-kabuk{display:grid;gap:clamp(14px,2.2vw,26px)}
.sinema-kare{position:relative;overflow:hidden;height:clamp(340px,72svh,620px);border-radius:var(--r-4);border:1px solid color-mix(in srgb,var(--accent) 22%,var(--cizgi));background:radial-gradient(120% 90% at 74% 12%,color-mix(in srgb,var(--accent) 30%,var(--bg)),var(--bg) 62%);isolation:isolate}
.sinema-ic{position:absolute;inset:-8% 0;transform:translateY(calc(var(--kaydir,0px) * .12));will-change:transform}
.sinema-ic img{width:100%;height:100%;object-fit:cover;object-position:50% 45%;animation:kenburns 32s ease-in-out infinite alternate;filter:saturate(1.02) contrast(1.04)}
.sinema-ic.doku{background:radial-gradient(80% 70% at 68% 26%,color-mix(in srgb,var(--accent) 44%,transparent),transparent 66%),repeating-linear-gradient(90deg,color-mix(in srgb,var(--ink) 6%,transparent) 0 1px,transparent 1px 92px)}
.sinema-perde{position:absolute;inset:0;background:linear-gradient(180deg,color-mix(in srgb,var(--bg) 12%,transparent),color-mix(in srgb,var(--bg) 78%,transparent) 94%);pointer-events:none}
.sinema-yazi{position:absolute;left:clamp(18px,4.4vw,64px);right:clamp(18px,4.4vw,64px);bottom:clamp(20px,4.6vw,58px);z-index:2;max-width:min(620px,90%)}
.sinema-yazi .ince{color:color-mix(in srgb,var(--accent-2) 82%,var(--ink));letter-spacing:.24em}
.sinema-yazi h2{font-family:var(--f-baslik);font-weight:600;font-size:clamp(26px,3.6vw,46px);line-height:1.06;letter-spacing:-.02em;margin:10px 0 0;color:var(--ink)}
.sinema-yazi p{margin:12px 0 0;font-size:clamp(13px,1.15vw,15.5px);line-height:1.7;color:color-mix(in srgb,var(--ink) 70%,transparent);max-width:52ch}
.sinema-kare::after{content:"";position:absolute;left:0;right:0;bottom:0;height:1px;background:linear-gradient(90deg,transparent,var(--accent-2),transparent);opacity:.55;z-index:3}
@media (max-width:1000px){.sinema-kare{height:clamp(300px,62svh,460px);border-radius:var(--r-3)}.sinema-yazi{left:18px;right:18px;bottom:18px}}
@media (prefers-reduced-motion:reduce){.sinema-ic{transform:none}.sinema-ic img{animation:none}}
"#;

/// Sayfa davranışının acil katmanı: kaydırıcı, parallax, mobil bar ve asistan.
const EK_JS: &str = r##"<script>
(function () {
  var azalt = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  // 1) Yorum kaydırıcısı: oklar, noktalar ve otomatik geçiş.
  var iz = document.getElementById("yorum-iz");
  if (iz && iz.children.length) {
    var kartlar = [].slice.call(iz.children);
    var noktalar = document.getElementById("yorum-noktalar");
    var adim = function () { return kartlar[0].offsetWidth + 18; };
    var sira = function () { return Math.round(iz.scrollLeft / Math.max(1, adim())); };
    var kaydir = function (i) {
      var hedef = kartlar[((i % kartlar.length) + kartlar.length) % kartlar.length];
      if (hedef) iz.scrollTo({ left: hedef.offsetLeft - iz.offsetLeft, behavior: azalt ? "auto" : "smooth" });
    };
    var isaretle = function () {
      if (!noktalar) return;
      var aktif = sira();
      [].forEach.call(noktalar.children, function (b, i) { b.className = i === aktif ? "etkin" : ""; });
    };
    if (noktalar) {
      kartlar.forEach(function (_, i) {
        var b = document.createElement("button");
        b.type = "button";
        b.setAttribute("aria-label", (i + 1) + ". yorum");
        b.addEventListener("click", function () { kaydir(i); });
        noktalar.appendChild(b);
      });
    }
    [].forEach.call(document.querySelectorAll("[data-yon]"), function (b) {
      b.addEventListener("click", function () { kaydir(sira() + Number(b.getAttribute("data-yon"))); });
    });
    iz.addEventListener("scroll", function () { window.requestAnimationFrame(isaretle); }, { passive: true });
    isaretle();
    if (!azalt) {
      var zaman = setInterval(function () {
        if (document.hidden) return;
        kaydir(sira() + 1);
      }, 6500);
      iz.addEventListener("mouseenter", function () { clearInterval(zaman); });
      iz.addEventListener("touchstart", function () { clearInterval(zaman); }, { passive: true });
    }
  }

  // 2) Mobil acil barı varsa gövdeye ek pay bırakılır.
  if (document.querySelector(".mobil-bar")) document.body.classList.add("acil-bar");

  // 3) Parallax: fotoğraf ve galeri kareleri kaydırmayla hafifçe kayar.
  if (!azalt) {
    var kayanlar = [].slice.call(document.querySelectorAll(".sahne.foto, .galeri-kare, .sinema-kare"));
    if (kayanlar.length) {
      var bekliyor = false;
      window.addEventListener("scroll", function () {
        if (bekliyor) return;
        bekliyor = true;
        window.requestAnimationFrame(function () {
          var d = window.scrollY;
          kayanlar.forEach(function (k) { k.style.setProperty("--kaydir", (d * .055).toFixed(2) + "px"); });
          bekliyor = false;
        });
      }, { passive: true });
    }
  }

  // 4) Yol yardım asistanı: durum + konum → WhatsApp.
  var asistan = document.getElementById("asistan");
  if (asistan) {
    var panel = document.getElementById("asistan-panel");
    var dugme = document.getElementById("asistan-dugme");
    var kapat = document.getElementById("asistan-kapat");
    var gonder = document.getElementById("asistan-gonder");
    var girdi = document.getElementById("asistan-not-girdi");
    var not = document.getElementById("asistan-not");
    var durum = "";
    var acKapa = function () {
      if (!panel) return;
      panel.hidden = !panel.hidden;
      if (dugme) dugme.setAttribute("aria-expanded", String(!panel.hidden));
      if (!panel.hidden && not) not.textContent = "";
    };
    if (dugme) dugme.addEventListener("click", acKapa);
    if (kapat) kapat.addEventListener("click", acKapa);
    [].forEach.call(asistan.querySelectorAll("[data-durum]"), function (b) {
      b.addEventListener("click", function () {
        durum = b.getAttribute("data-durum") || "";
        [].forEach.call(asistan.querySelectorAll("[data-durum]"), function (x) { x.classList.toggle("secili", x === b); });
      });
    });
    var yolla = function (konum) {
      var wa = asistan.getAttribute("data-wa");
      if (!wa) return;
      var metin = "Merhaba, yolda kaldım. Konumum: ";
      if (konum) metin += konum;
      if (durum) metin += " · " + durum;
      var ek = girdi && girdi.value.trim();
      if (ek) metin += " · " + ek;
      window.open("https://wa.me/" + wa + "?text=" + encodeURIComponent(metin), "_blank", "noopener");
      if (not) not.textContent = "WhatsApp açılıyor… Konumu ataç → Konum ile de ekleyebilirsiniz.";
    };
    if (gonder) {
      gonder.addEventListener("click", function () {
        if (not) not.textContent = "Konum alınıyor…";
        if (!navigator.geolocation) { yolla(null); return; }
        var bitti = false;
        navigator.geolocation.getCurrentPosition(
          function (k) {
            if (bitti) return;
            bitti = true;
            yolla("https://maps.google.com/?q=" + k.coords.latitude.toFixed(5) + "," + k.coords.longitude.toFixed(5));
          },
          function () { if (!bitti) { bitti = true; yolla(null); } },
          { timeout: 3500, maximumAge: 60000 }
        );
      });
    }
  }
})();
</script>
"##;

// ---------------------------------------------------------------------------
// Ajans profili: beş bölümlü sinematik kompozisyon
// ---------------------------------------------------------------------------

fn ajans_satirlar(metin: &str, ilk: &str, ikinci: &str) -> (String, String) {
    let parcalar: Vec<&str> = metin
        .split(['\n', '|'])
        .map(str::trim)
        .filter(|parca| !parca.is_empty())
        .collect();
    match parcalar.as_slice() {
        [] => (ilk.to_string(), ikinci.to_string()),
        [tek] => (tek.to_string(), String::new()),
        [a, b, ..] => (a.to_string(), b.to_string()),
    }
}

fn ajans_kelime(metin: &str, baslangic: usize) -> (String, usize) {
    let mut html = String::new();
    let mut sira = baslangic;
    for kelime in metin.split_whitespace() {
        if !html.is_empty() {
            html.push(' ');
        }
        html.push_str(&format!(
            r##"<span class="aj-kelime" style="--i:{sira}"><i>{}</i></span>"##,
            kacis(kelime)
        ));
        sira += 1;
    }
    (html, sira)
}

fn ajans_rozetler(g: &Girdi, brif: Option<&Brif>) -> String {
    let metinler: Vec<String> = if let Some(kalemler) = brif
        .and_then(|b| b.bolum(0))
        .map(|b| b.kalemler.as_slice())
        .filter(|kalemler| !kalemler.is_empty())
    {
        kalemler
            .iter()
            .take(3)
            .map(|kalem| {
                if !kalem.ad.trim().is_empty() {
                    kalem.ad.clone()
                } else {
                    kalem.aciklama.clone()
                }
            })
            .collect()
    } else if let Some(tasarim) = g.tasarim.as_ref() {
        tasarim
            .rozetler
            .iter()
            .map(|metin| (*metin).to_string())
            .collect()
    } else {
        vec![
            "Yerel ekip".to_string(),
            "Doğrudan iletişim".to_string(),
            "Tek dokunuş".to_string(),
        ]
    };
    let mut html = String::new();
    for (sira, metin) in metinler.iter().take(3).enumerate() {
        html.push_str(&format!(
            r##"<span class="aj-rozet{}">{}{}</span>"##,
            if sira == 0 { " canli" } else { "" },
            ikon(["kalkan", "saat", "kivilcim"][sira]),
            kacis(metin)
        ));
    }
    html
}

fn ajans_foto_not(g: &Girdi) -> String {
    let cumle = |atif: &Option<String>| -> String {
        match atif {
            Some(atif) if !atif.trim().is_empty() => {
                let metin = kacis(atif.trim());
                if metin.starts_with("Fotoğraf") {
                    format!(" {metin}.")
                } else {
                    format!(" Fotoğraf: {metin}.")
                }
            }
            _ => String::new(),
        }
    };
    if g.foto_atif == g.galeri_atif {
        cumle(&g.foto_atif)
    } else {
        format!("{}{}", cumle(&g.foto_atif), cumle(&g.galeri_atif))
    }
}

fn asistan_html(acil: bool, wa_no: &str) -> String {
    if !acil || wa_no.is_empty() {
        return String::new();
    }
    format!(
        r##"<div class="asistan" id="asistan" data-wa="{wa_no}">
<button class="asistan-dugme" id="asistan-dugme" type="button" aria-expanded="false" aria-controls="asistan-panel"><span class="asistan-nokta"></span>Yol Yardım Asistanı</button>
<div class="asistan-panel" id="asistan-panel" hidden>
<div class="asistan-ust"><b>Yol Yardım Asistanı</b><button class="asistan-kapat" id="asistan-kapat" type="button" aria-label="Kapat">×</button></div>
<p class="asistan-soru">Durumunuzu seçin, en yakın ekibi yönlendirelim.</p>
<div class="asistan-secim" role="group" aria-label="Acil durum">
<button type="button" data-durum="Arıza">🔧 Arıza</button>
<button type="button" data-durum="Kaza">🚨 Kaza</button>
<button type="button" data-durum="Lastik">🛞 Lastik</button>
<button type="button" data-durum="Akü">🔋 Akü</button>
</div>
<label class="asistan-alan"><span>Konum notu (isteğe bağlı)</span><input id="asistan-not-girdi" type="text" placeholder="Örn. otoban çıkışı, benzinlik yanı" autocomplete="off"></label>
<button class="asistan-gonder" id="asistan-gonder" type="button">Konumumu gönder</button>
<p class="asistan-not" id="asistan-not" role="status" aria-live="polite"></p>
</div>
</div>"##
    )
}

/// 1240 → 1.240. Sayaçların JS'siz de okunur kalması için.
/// Demo sayfasını diske yazmadan önce küçültür: HTML ile birlikte gömülü
/// CSS ve JS de sıkıştırılır. Kapanış etiketleri korunur (karne rozeti
/// `</body>` arar). Küçültme başarısız olursa ham HTML döner; üretim durmaz.
pub fn kucult(html: &str) -> String {
    let ayar = minify_html::Cfg {
        keep_closing_tags: true,
        keep_html_and_head_opening_tags: true,
        keep_comments: false,
        minify_css: true,
        minify_js: true,
        ..Default::default()
    };
    String::from_utf8(minify_html::minify(html.as_bytes(), &ayar))
        .unwrap_or_else(|_| html.to_string())
}

fn binlik(deger: i64) -> String {
    let metin = deger.abs().to_string();
    let mut cikti = String::new();
    for (sira, harf) in metin.chars().enumerate() {
        if sira > 0 && (metin.len() - sira).is_multiple_of(3) {
            cikti.push('.');
        }
        cikti.push(harf);
    }
    if deger < 0 {
        format!("-{cikti}")
    } else {
        cikti
    }
}

/// Güven ve dönüşüm bölümü: memnuniyet sayaçları, rozetler, şeffaf hizmet
/// kademeleri ve etkileşimli SSS akordeonu. İçerik sektör kimliğine göre
/// değişir; amaç abartı değil, karar vermeyi kolaylaştıran netlik.
fn guven_html(g: &Girdi, acil: bool, bolge: &str, cta: &str, wa_no: &str) -> String {
    let grup = g
        .tasarim
        .as_ref()
        .map(|tasarim| tasarim.grup.as_str())
        .unwrap_or(if acil { "acil" } else { "" });
    let luks = matches!(grup, "mutfak" | "konaklama" | "guzellik" | "etkinlik");
    let temiz = matches!(grup, "saglik" | "kurumsal" | "egitim");
    let (baslik, alt) = if acil {
        (
            "Yolda kalmak, tek bir karar.",
            "Hız kadar netlik de gerekir: ortalama varış, uzman ekip ve kapsam tek bakışta görünür.",
        )
    } else if luks {
        (
            "Güven, sessizce kurulur.",
            "Ölçülebilir memnuniyet, aynı ekibin sürekliliği ve şeffaf kademeler; karar vermeden önce her ayrıntı açık.",
        )
    } else if temiz {
        (
            "Karar öncesi net bir çerçeve.",
            "Memnuniyet oranı, uzmanlık yılı ve dönüş süresi; hepsi aynı tabloda ve doğrulanabilir.",
        )
    } else {
        (
            "Güven, ölçülebilir bir söz.",
            "Sayaçlar, rozetler ve şeffaf kademeler; hizmetin kalitesi ilk temastan önce görünür.",
        )
    };
    let sayaclar: &[(f32, &str, &str)] = if acil {
        &[
            (12.0, "dk", "Ortalama varış"),
            (98.0, "%", "Memnuniyet"),
            (24.0, "/7", "Kesintisiz hat"),
            (4.9, "★", "Müşteri puanı"),
        ]
    } else if luks {
        &[
            (4.9, "★", "Misafir puanı"),
            (18.0, "yıl", "Deneyim"),
            (96.0, "%", "Tekrar tercih"),
            (1240.0, "+", "Değerlendirme"),
        ]
    } else if temiz {
        &[
            (98.0, "%", "Memnuniyet"),
            (14.0, "yıl", "Uzmanlık"),
            (24.0, "sa", "İçinde dönüş"),
            (2400.0, "+", "Tamamlanan iş"),
        ]
    } else {
        &[
            (98.0, "%", "Memnuniyet"),
            (12.0, "yıl", "Deneyim"),
            (2400.0, "+", "Tamamlanan iş"),
            (24.0, "sa", "İçinde dönüş"),
        ]
    };
    let rozetler: &[&str] = if acil {
        &[
            "7/24 Kesintisiz Hizmet",
            "Uzman Ekip",
            "Şeffaf Fiyat",
            "Sigortalı Süreç",
        ]
    } else if temiz {
        &[
            "7/24 Kesintisiz Hizmet",
            "Uzman Ekip",
            "Şeffaf Kapsam",
            "Veriyle Takip",
        ]
    } else {
        &[
            "7/24 Kesintisiz Hizmet",
            "Uzman Ekip",
            "Şeffaf Kademeler",
            "Kişisel Takip",
        ]
    };
    let kademeler: &[(&str, &str, &str, &[&str], bool)] = if acil {
        &[
            (
                "Tek çağrı",
                "Anlık",
                "Yolda kalma anında tek kapsam: çekici, yol yardımı ve güvenli teslim.",
                &["7/24 canlı hat", "Konum paylaşımı", "Sabit fiyat teyidi"],
                false,
            ),
            (
                "Sözleşmeli",
                "Aylık",
                "İşletmeler için öncelikli sıra, tek fatura ve düzenli rapor.",
                &["Öncelikli müdahale", "Aylık rapor", "Tek fatura"],
                true,
            ),
            (
                "Filo",
                "Yıllık",
                "Araç filosu için ayrılmış ekip ve yazılı taahhüt.",
                &["Ayrılmış ekip", "SLA taahhüdü", "Yerinde bakım"],
                false,
            ),
        ]
    } else if luks {
        &[
            (
                "Deneme",
                "İlk temas",
                "Kısa bir tanışma, ihtiyaç analizi ve net bir ilk öneri.",
                &["Kısa görüşme", "Kişisel plan", "Yazılı teklif"],
                false,
            ),
            (
                "İmza",
                "Dönemsel",
                "Markaya ait süreklilik: aynı ekip, aynı özen, aynı ritim.",
                &["Ayrılmış ekip", "Öncelikli takvim", "Aylık rapor"],
                true,
            ),
            (
                "Özel",
                "Yıllık",
                "Uzun soluklu iş birliği ve konsept danışmanlığı.",
                &["Konsept desteği", "Etkinlik planı", "Özel iletişim hattı"],
                false,
            ),
        ]
    } else if temiz {
        &[
            (
                "Tanışma",
                "Tek görüşme",
                "İhtiyaç ve beklenti baştan konuşulur, kapsam yazılı hale gelir.",
                &["İhtiyaç analizi", "Yazılı kapsam", "Net zaman planı"],
                false,
            ),
            (
                "Süreklilik",
                "Aylık",
                "Düzenli takip, ölçülebilir çıktı ve tek muhatap.",
                &["Aylık rapor", "Öncelikli dönüş", "Tek muhatap"],
                true,
            ),
            (
                "Kurumsal",
                "Yıllık",
                "Ekipler için tanımlı süreç, eğitim ve raporlama.",
                &["Süreç tasarımı", "Ekip eğitimi", "Yönetim raporu"],
                false,
            ),
        ]
    } else {
        &[
            (
                "Temel",
                "Tek seferlik",
                "Net kapsam, sabit fiyat ve tek seferlik çözüm.",
                &["İhtiyaç analizi", "Sabit fiyat", "Teslim garantisi"],
                false,
            ),
            (
                "Standart",
                "Aylık",
                "Planlı takip ve öncelikli iletişim.",
                &["Planlı takip", "Öncelikli hat", "Aylık özet"],
                true,
            ),
            (
                "Öncelikli",
                "Yıllık",
                "Ayrılmış ekip ve tanımlı yanıt süresi.",
                &["Ayrılmış ekip", "Yanıt taahhüdü", "Yerinde destek"],
                false,
            ),
        ]
    };
    let sss_kalemleri: Vec<(String, String)> = if acil {
        vec![
            (
                "Ne kadar sürede yanınızda oluyorsunuz?".to_string(),
                "Şehir merkezinde ortalama 15–20 dakika. Konum WhatsApp üzerinden anında ekibe düşer; varış süresi canlı olarak paylaşılır.".to_string(),
            ),
            (
                "Gece, hafta sonu ve tatil günleri de çalışıyor musunuz?".to_string(),
                "Evet. Hat 7/24 açık; gece ve tatil günlerinde ayrı bir nöbetçi ekip devrede.".to_string(),
            ),
            (
                "Çekici ve teslim süreci nasıl ilerliyor?".to_string(),
                "Ekip önce güvenliği sağlar, ardından aracın durumunu fotoğraflar. Teslim adresi ve saat bilgisi tek mesajda onaylanır.".to_string(),
            ),
            (
                format!("{bolge} dışına hizmet veriyor musunuz?"),
                format!("{bolge} merkezli çalışıyoruz; çevre ilçelere planlı yönlendirme yapılır. Kapsam, konum paylaşıldığında netleşir."),
            ),
        ]
    } else {
        vec![
            (
                "İlk temas nasıl ilerliyor?".to_string(),
                "Kısa bir görüşmeyle ihtiyaç dinlenir; ardından kapsam, zaman planı ve fiyat tek sayfada yazılı olarak paylaşılır.".to_string(),
            ),
            (
                "Fiyatlandırma neye göre belirleniyor?".to_string(),
                "Kapsam, süre ve ekip büyüklüğüne göre. Kalemler baştan listelenir; sürpriz ek ücret çıkmaz.".to_string(),
            ),
            (
                "Hangi bölgede hizmet veriyorsunuz?".to_string(),
                format!("{bolge} ve çevresinde hizmet veriyoruz. Farklı bir bölge için uzaktan destek ve planlı ziyaret seçenekleri sunulur."),
            ),
            (
                "Randevu ve iptal koşulları nasıl işliyor?".to_string(),
                "Randevular 24 saat öncesine kadar ücretsiz güncellenebilir. Değişiklikler aynı kanal üzerinden tek mesajla onaylanır.".to_string(),
            ),
        ]
    };
    let mut sayac_html = String::new();
    for (sira, (deger, ek, etiket)) in sayaclar.iter().enumerate() {
        let ondalik = if deger.fract() > 0.0 { 1 } else { 0 };
        let metin = if ondalik > 0 {
            format!("{deger:.1}").replace('.', ",")
        } else {
            binlik(*deger as i64)
        };
        sayac_html.push_str(&format!(
            r##"<div class="aj-sayac" data-aj-reveal style="--i:{sira}"><b><span data-aj-sayac="{deger}" data-aj-ondalik="{ondalik}">{metin}</span><i>{ek}</i></b><span>{etiket}</span></div>"##
        ));
    }
    let mut rozet_html = String::new();
    let rozet_simgeleri = ["kalkan", "ekip", "makbuz", "odul"];
    for (sira, rozet) in rozetler.iter().enumerate() {
        rozet_html.push_str(&format!(
            r##"<span class="aj-guven-rozet">{}{}</span>"##,
            ikon(rozet_simgeleri[sira % rozet_simgeleri.len()]),
            kacis(rozet)
        ));
    }
    let mut kademe_html = String::new();
    for (sira, (ad, donem, aciklama, maddeler, one)) in kademeler.iter().enumerate() {
        let mut liste = String::new();
        for madde in maddeler.iter() {
            liste.push_str(&format!("<li>{}{}</li>", ikon("tik"), kacis(madde)));
        }
        let vurgu = if *one {
            "<b>En çok tercih edilen</b>"
        } else {
            ""
        };
        kademe_html.push_str(&format!(
            r##"<article class="aj-kademe aj-cam{sinif}" data-aj-tilt data-aj-reveal style="--i:{sira}"><div class="aj-kademe-ust"><span>{donem}</span>{vurgu}</div><h3>{ad}</h3><p>{aciklama}</p><ul>{liste}</ul><a class="aj-kademe-cta" href="#iletisim">Teklif al <b>↗</b></a></article>"##,
            sinif = if *one { " aj-kademe-one" } else { "" },
            donem = kacis(donem),
            ad = kacis(ad),
            aciklama = kacis(aciklama)
        ));
    }
    let mut sss_html = String::new();
    for (soru, cevap) in &sss_kalemleri {
        sss_html.push_str(&format!(
            r##"<details class="aj-sss-madde"><summary>{}</summary><p>{}</p></details>"##,
            kacis(soru),
            kacis(cevap)
        ));
    }
    let hedef = if !wa_no.is_empty() {
        format!(
            "https://wa.me/{wa_no}?text={}",
            url_kacis("Merhaba, hizmet kademeleri hakkında bilgi almak istiyorum.")
        )
    } else {
        g.telefon
            .as_ref()
            .map(|tel| format!("tel:+{}", tel.trim_start_matches('+')))
            .unwrap_or_else(|| "#iletisim".to_string())
    };
    format!(
        r####"<section class="aj-bolum aj-guven" id="guven" data-aj-section>
<div class="kabuk">
<div class="aj-bolum-ust"><div class="aj-bolum-yan"><span class="aj-no">04</span><span class="aj-dikey">Güven</span></div><div><span class="aj-eyebrow">{kalkan}Güven &amp; dönüşüm</span><h2 data-aj-reveal>{baslik}</h2><p data-aj-reveal>{alt}</p></div></div>
<div class="aj-sayaclar">{sayaclar}</div>
<div class="aj-guven-rozetler">{rozetler}</div>
<div class="aj-kademe-baslik"><span class="aj-eyebrow">Hizmet kademeleri</span><p>Kapsam baştan netleşir, sürpriz kalem olmaz.</p></div>
<div class="aj-kademeler">{kademeler}</div>
<div class="aj-sss-baslik"><span class="aj-eyebrow">Sık sorulanlar</span><h3>Karar vermeden önce.</h3></div>
<div class="aj-sss">{sss}</div>
<div class="aj-guven-cta"><a class="buton buton-dolu" data-aj-magnetic href="{hedef}">{cta} <b>↗</b></a><a class="aj-metin-link" href="#iletisim">Formu doldur <span>↘</span></a></div>
</div>
</section>
"####,
        baslik = kacis(baslik),
        alt = kacis(alt),
        sayaclar = sayac_html,
        rozetler = rozet_html,
        kademeler = kademe_html,
        sss = sss_html,
        hedef = hedef,
        kalkan = ikon("kalkan"),
        cta = kacis(cta)
    )
}

fn ajans_sayfa(g: &Girdi, ic: &Icerik) -> String {
    let brif = g.brif.as_ref();
    let tasarim = g.tasarim.as_ref();
    let ad = kacis(&g.ad);
    let sektor = kacis(&g.sektor);
    let adres = kacis(&g.adres);
    let bolge = ilce_metni(&g.adres);
    let harf =
        g.ad.chars()
            .next()
            .map(|harf| harf.to_uppercase().to_string())
            .unwrap_or_else(|| "•".to_string());
    let tel_hedef = g
        .telefon
        .as_ref()
        .map(|tel| format!("tel:+{}", tel.trim_start_matches('+')));
    let wa_no: String = g
        .telefon
        .as_deref()
        .or(g.satici_tel.as_deref())
        .map(|tel| tel.chars().filter(char::is_ascii_digit).collect())
        .unwrap_or_default();
    let acil = acil_sektor(&g.sektor) || tasarim.is_some_and(|t| t.aciliyet >= 2);
    let stil = brif
        .map(|b| b.stil.as_str())
        .filter(|stil| !stil.is_empty())
        .or_else(|| tasarim.map(|t| crate::tasarim_brifi::stil_adi(&t.grup, t.aciliyet >= 2)))
        .unwrap_or("luxe-sinema");
    let grup = tasarim
        .map(|tasarim| tasarim.grup.as_str())
        .unwrap_or(if acil { "acil" } else { "" });
    let hizmet_simgeleri = hizmet_ikonlari(grup);
    let hareket = brif
        .map(|b| b.hareket.as_str())
        .filter(|metin| !metin.is_empty())
        .or_else(|| {
            tasarim
                .map(|t| t.hareket.as_str())
                .filter(|metin| !metin.is_empty())
        })
        .unwrap_or("canli");
    let vurgu = brif
        .map(|b| b.palet.vurgu.as_str())
        .filter(|renk| !renk.is_empty())
        .unwrap_or(g.vurgu.as_str());
    let vurgu2 = brif
        .map(|b| b.palet.vurgu2.as_str())
        .filter(|renk| !renk.is_empty())
        .or_else(|| {
            tasarim
                .map(|t| t.vurgu2.as_str())
                .filter(|renk| !renk.is_empty())
        })
        .unwrap_or(vurgu);
    let zemin = brif
        .map(|b| b.palet.zemin.as_str())
        .filter(|renk| !renk.is_empty())
        .unwrap_or(g.zemin.as_str());
    let yazi = brif
        .map(|b| b.palet.yazi.as_str())
        .filter(|renk| !renk.is_empty())
        .unwrap_or(g.yazi.as_str());
    let serif = matches!(
        g.tur,
        "menu" | "kafe" | "unlu" | "galeri" | "klinik" | "otel" | "eglence"
    );
    let baslik_yigin = brif
        .map(|b| b.tipografi.baslik.as_str())
        .filter(|font| !font.is_empty())
        .or_else(|| {
            tasarim
                .map(|t| t.baslik.as_str())
                .filter(|font| !font.is_empty())
        })
        .unwrap_or(if serif {
            "'Instrument Serif','Space Grotesk',Georgia,'Times New Roman',serif"
        } else {
            "'Space Grotesk','Geist',system-ui,-apple-system,'Segoe UI',sans-serif"
        });
    let govde_yigin = brif
        .map(|b| b.tipografi.govde.as_str())
        .filter(|font| !font.is_empty())
        .or_else(|| {
            tasarim
                .map(|t| t.govde.as_str())
                .filter(|font| !font.is_empty())
        })
        .unwrap_or("'Geist',system-ui,-apple-system,'Segoe UI',sans-serif");
    let cta = if !g.cta.trim().is_empty() {
        g.cta.trim().to_string()
    } else if let Some(b) = brif.filter(|b| !b.cta.trim().is_empty()) {
        b.cta.trim().to_string()
    } else if let Some(t) = tasarim.filter(|t| !t.cta.trim().is_empty()) {
        t.cta.trim().to_string()
    } else {
        ic.cta.to_string()
    };
    let slogan = if !g.slogan.trim().is_empty() {
        g.slogan.trim().to_string()
    } else if let Some(b) = brif
        .and_then(|b| b.bolum(0))
        .filter(|b| !b.metin.trim().is_empty())
    {
        b.metin.clone()
    } else {
        ic.alt.to_string()
    };
    let bolum_metni = ozel(&g.metin.bolum_baslik, ic.bolum);
    let hikaye_baslik = ozel(&g.metin.hakkinda_baslik, ic.hikaye_baslik);
    let hikaye1 = ozel(&g.metin.hakkinda1, ic.hikaye[0]);
    let hikaye2 = ozel(&g.metin.hakkinda2, ic.hikaye[1]);
    let b0 = brif.and_then(|b| b.bolum(0));
    let (h1_text, h2_text) = if let Some(b) = b0.filter(|b| !b.baslik.trim().is_empty()) {
        ajans_satirlar(&b.baslik, "", "")
    } else {
        (
            ozel(&g.metin.manset1, ic.manset[0]),
            ozel(&g.metin.manset2, ic.manset[1]),
        )
    };
    let (h1, kelime) = ajans_kelime(&h1_text, 0);
    let (h2, _) = ajans_kelime(&h2_text, kelime);
    let hero_etiket = b0
        .map(|b| b.etiket.trim())
        .filter(|metin| !metin.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{sektor} · {bolge}"));
    let hero_alt = if let Some(b) = b0.filter(|b| !b.metin.trim().is_empty()) {
        b.metin.clone()
    } else {
        slogan.clone()
    };
    let hero_rozetler = ajans_rozetler(g, brif);
    let hero_gorsel = match &g.foto {
        Some((mime, veri)) => format!(
            r##"<img class="aj-hero-foto" src="data:{mime};base64,{veri}" alt="{ad}" loading="eager" decoding="async">"##
        ),
        None => format!(
            r##"<div class="aj-doku aj-doku-hero"><span>{}</span></div>"##,
            kacis(&harf)
        ),
    };
    let puan_kart = match (&g.puan, &g.yorum) {
        (Some(puan), Some(yorum)) => format!(
            r##"<div class="aj-cam aj-puan"><div class="aj-yildiz">{YILDIZ}{YILDIZ}{YILDIZ}{YILDIZ}{YILDIZ}</div><b>{puan}</b><span>{yorum}</span></div>"##
        ),
        (Some(puan), None) => format!(
            r##"<div class="aj-cam aj-puan"><div class="aj-yildiz">{YILDIZ}{YILDIZ}{YILDIZ}{YILDIZ}{YILDIZ}</div><b>{puan}</b></div>"##
        ),
        _ => String::new(),
    };
    let birincil = match &tel_hedef {
        Some(hedef) => format!(
            r##"<a class="buton buton-dolu aj-buton" data-aj-magnetic href="{hedef}">{TELEFON_IKON}<span>{}</span><b>↗</b></a>"##,
            kacis(&cta)
        ),
        None => String::new(),
    };
    let konum_buton = if acil && !wa_no.is_empty() {
        format!(
            r##"<a class="buton buton-konum" href="https://wa.me/{wa_no}?text={}" target="_blank" rel="noopener">{KONUM_IKON}Canlı konum gönder</a>"##,
            url_kacis("Merhaba, yolda kaldım. Konumum: ")
        )
    } else {
        String::new()
    };
    let ikincil = format!(
        r##"<a class="aj-metin-link" href="#hakkimizda">{} <span>↘</span></a>"##,
        ozel(&g.ikincil, "Yaklaşımı gör")
    );

    let b1 = brif.and_then(|b| b.bolum(1));
    let manifesto_etiket = b1
        .map(|b| b.etiket.trim())
        .filter(|metin| !metin.is_empty())
        .unwrap_or("Manifesto");
    let manifesto_baslik = b1
        .map(|b| b.baslik.trim())
        .filter(|metin| !metin.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| hikaye_baslik.clone());
    let manifesto_buyuk = b1
        .map(|b| b.metin.trim())
        .filter(|metin| !metin.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| hikaye1.clone());
    let manifesto_gorsel = match &g.hikaye_foto {
        Some((mime, veri)) => format!(
            r##"<img src="data:{mime};base64,{veri}" alt="{ad} — {sektor}" loading="lazy" decoding="async">"##
        ),
        None => format!(
            r##"<div class="aj-doku"><span>{}</span></div>"##,
            kacis(&harf)
        ),
    };

    let b2 = brif.and_then(|b| b.bolum(2));
    let hizmet_etiket = b2
        .map(|b| b.etiket.trim())
        .filter(|metin| !metin.is_empty())
        .unwrap_or(ic.etiket);
    let hizmet_baslik = b2
        .map(|b| b.baslik.trim())
        .filter(|metin| !metin.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| bolum_metni.clone());
    let hizmet_alt = b2
        .map(|b| b.metin.trim())
        .filter(|metin| !metin.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| ozel(&g.metin.bolum_alt, ic.bolum_alt));
    let hizmetler: Vec<(String, String)> = if let Some(kalemler) = b2
        .map(|b| b.kalemler.as_slice())
        .filter(|kalemler| !kalemler.is_empty())
    {
        kalemler
            .iter()
            .take(4)
            .map(|kalem| (kalem.ad.clone(), kalem.aciklama.clone()))
            .collect()
    } else {
        ic.kalemler
            .iter()
            .enumerate()
            .map(|(sira, (baslik, aciklama))| {
                (
                    ozel(g.metin.satir(&g.metin.kalemler, sira, 0), baslik),
                    ozel(g.metin.satir(&g.metin.kalemler, sira, 1), aciklama),
                )
            })
            .collect()
    };
    let mut hizmet_kartlari = String::new();
    for (sira, (baslik, aciklama)) in hizmetler.iter().take(4).enumerate() {
        hizmet_kartlari.push_str(&format!(
            r##"<article class="aj-hizmet aj-cam" data-aj-tilt data-aj-reveal style="--i:{sira}">
<div class="aj-hizmet-ust"><span class="aj-hizmet-no">0{}</span><span class="aj-hizmet-ikon">{simge}</span></div><h3>{}</h3><p>{}</p><span class="aj-hizmet-ok">↗</span>
</article>"##,
            sira + 1,
            kacis(baslik),
            kacis(aciklama),
            simge = ikon(hizmet_simgeleri[sira % hizmet_simgeleri.len()])
        ));
    }

    // Bento ızgarasının alt şeridi: üç mikro bilgi barı.
    let yanit_metni = if acil {
        "7/24 · ortalama 25 dk"
    } else {
        "Aynı gün dönüş"
    };
    hizmet_kartlari.push_str(&format!(
        r##"<div class="aj-mikro-bar" data-aj-reveal><div class="aj-mikro"><span>Yanıt</span><b>{yanit}</b></div><div class="aj-mikro"><span>Bölge</span><b>{bolge}</b></div><div class="aj-mikro"><span>Kapsam</span><b>Yazılı teklif · sabit kapsam</b></div></div>"##,
        yanit = kacis(yanit_metni),
        bolge = kacis(&bolge)
    ));

    let b_ai = brif.and_then(|b| b.bolum(3));
    let ai_etiket = b_ai
        .map(|b| b.etiket.trim())
        .filter(|metin| !metin.is_empty())
        .unwrap_or("AI Asistan Widget / Acil WhatsApp");
    let ai_baslik = b_ai
        .map(|b| b.baslik.trim())
        .filter(|metin| !metin.is_empty())
        .unwrap_or("Bir mesaj kadar\nyakınınızdayız.");
    let ai_metin = b_ai
        .map(|b| b.metin.trim())
        .filter(|metin| !metin.is_empty())
        .unwrap_or(
            "Demo asistanı sorularınızı yanıtlar; devam etmek isterseniz WhatsApp hattı hazır.",
        );
    let ai_kalemler: Vec<(String, String)> = b_ai
        .map(|b| b.kalemler.as_slice())
        .filter(|kalemler| !kalemler.is_empty())
        .map(|kalemler| {
            kalemler
                .iter()
                .take(3)
                .map(|kalem| (kalem.ad.clone(), kalem.aciklama.clone()))
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                (
                    "7/24 AI asistan".to_string(),
                    "Soruları anlar, doğru adımı önerir.".to_string(),
                ),
                (
                    "Canlı WhatsApp".to_string(),
                    "Konuşmayı doğrudan ekibe bağlar.".to_string(),
                ),
                (
                    "Konum paylaşımı".to_string(),
                    "Tek dokunuşla iletilir.".to_string(),
                ),
            ]
        });
    let mut ai_kartlar = String::new();
    for (sira, (baslik, aciklama)) in ai_kalemler.iter().enumerate() {
        ai_kartlar.push_str(&format!(
            r##"<div class="aj-ai-kart" data-aj-reveal style="--i:{sira}"><b>{}</b><span>{}</span></div>"##,
            kacis(baslik),
            kacis(aciklama)
        ));
    }
    let ai_link = if !wa_no.is_empty() {
        let mesaj = if acil {
            "Merhaba, acil durum için yazıyorum."
        } else {
            "Merhaba, AI asistan üzerinden bilgi almak istiyorum."
        };
        format!(
            r##"<a class="aj-buyuk-link" data-aj-magnetic href="https://wa.me/{wa_no}?text={}">{} <span>↗</span></a>"##,
            url_kacis(mesaj),
            if acil {
                "Acil WhatsApp"
            } else {
                "WhatsApp'tan yaz"
            }
        )
    } else if let Some(hedef) = &tel_hedef {
        format!(
            r##"<a class="aj-buyuk-link" data-aj-magnetic href="{hedef}">{} <span>↗</span></a>"##,
            kacis(&cta)
        )
    } else {
        String::new()
    };

    // Galeri artık AI/asistan bölümünün ardından ayrı bir seçili işler
    // bölümü olarak çizilir; görsel hikâye kaybolmaz.
    let isler_etiket = "Seçili işler";
    let isler_baslik = "Seçili işler.";
    let isler_alt = if g.galeri_ornek {
        "Galeri, işletmenin kendi fotoğraflarıyla değiştirilir."
    } else {
        "İşletmenin kendi fotoğrafları."
    };
    let mut is_kartlari = String::new();
    for sira in 0..3 {
        let (varsayilan_baslik, varsayilan_aciklama) =
            ic.neden.get(sira).copied().unwrap_or(("", ""));
        let (baslik, aciklama) = (
            varsayilan_baslik.to_string(),
            varsayilan_aciklama.to_string(),
        );
        let gorsel = if g.bolumler.galeri {
            g.galeri.get(sira)
        } else {
            None
        };
        if let Some((mime, veri)) = gorsel {
            is_kartlari.push_str(&format!(
                r##"<figure class="aj-is" data-aj-reveal style="--i:{sira}">
<img src="data:{mime};base64,{veri}" alt="{ad} — {sektor}" loading="lazy" decoding="async">
<figcaption><span>0{}</span><b>{}</b><small>{}</small></figcaption>
</figure>"##,
                sira + 1,
                kacis(&baslik),
                kacis(&aciklama)
            ));
        } else {
            is_kartlari.push_str(&format!(
                r##"<figure class="aj-is aj-is-doku" data-aj-reveal style="--i:{sira}">
<div class="aj-doku"><span>{}</span></div>
<figcaption><span>0{}</span><b>{}</b><small>{}</small></figcaption>
</figure>"##,
                kacis(&harf),
                sira + 1,
                kacis(&baslik),
                kacis(&aciklama)
            ));
        }
    }

    let b4 = brif.and_then(|b| b.bolum(4));
    let iletisim_etiket = b4
        .map(|b| b.etiket.trim())
        .filter(|metin| !metin.is_empty())
        .unwrap_or("İletişim");
    let iletisim_baslik = b4
        .map(|b| b.baslik.trim())
        .filter(|metin| !metin.is_empty())
        .unwrap_or("Bir sonraki adım.");
    let iletisim_metin = b4
        .map(|b| b.metin.trim())
        .filter(|metin| !metin.is_empty())
        .unwrap_or("Projeyi konuşmak için kısa bir not bırakın; doğru ekiple tanışın.");
    let saatler = if g.bolumler.saatler {
        saatler_html(g)
    } else {
        String::new()
    };
    let harita = if g.bolumler.harita && g.konum.is_some() {
        harita_html(g, &ad)
    } else {
        String::new()
    };
    let tel_kart = match (&tel_hedef, &g.telefon_goster) {
        (Some(hedef), goster) => format!(
            r##"<a class="aj-ilet-kart" href="{hedef}">{TELEFON_IKON}<div><small>Telefon</small><b>{}</b></div></a>"##,
            kacis(goster)
        ),
        _ => String::new(),
    };
    let konum_kart = if !g.adres.trim().is_empty() {
        format!(
            r##"<div class="aj-ilet-kart">{KONUM_IKON}<div><small>Adres</small><b>{adres}</b></div></div>"##
        )
    } else {
        String::new()
    };
    let karne = match &g.karne {
        Some(adres) => format!(
            r##"<a href="{}">Dijital görünürlük karnesi</a>"##,
            kacis(adres)
        ),
        None => String::new(),
    };
    let foto_not = ajans_foto_not(g);
    let mobil = if !wa_no.is_empty() {
        let ara = match &tel_hedef {
            Some(hedef) => format!(
                r##"<a class="mobil-ara" href="{hedef}" aria-label="Ara">{TELEFON_IKON}<span>Ara</span></a>"##
            ),
            None => String::new(),
        };
        let mesaj = if acil {
            "Merhaba, yolda kaldım. Konumum: "
        } else {
            "Merhaba, bilgi almak istiyorum."
        };
        format!(
            r##"<div class="mobil-bar">{ara}<a class="mobil-konum" href="https://wa.me/{wa_no}?text={konum_mesaj}" target="_blank" rel="noopener">{KONUM_IKON}<span>{etiket}</span></a></div>"##,
            konum_mesaj = url_kacis(mesaj),
            etiket = if acil {
                "Canlı Konum Gönder"
            } else {
                "WhatsApp'tan yaz"
            }
        )
    } else {
        match &tel_hedef {
            Some(hedef) => format!(
                r##"<a class="mobil-cta" href="{hedef}">{TELEFON_IKON}{}</a>"##,
                kacis(&cta)
            ),
            None => String::new(),
        }
    };
    let asistan = asistan_html(acil, &wa_no);
    let akis = format!("<span>{}</span>", kacis(ic.akis)).repeat(10);
    let tema_acik = parlak_mi(zemin);
    let kose = match g.kose.trim() {
        "keskin" => " data-kose=\"keskin\"",
        "yumusak" => " data-kose=\"yumusak\"",
        _ => "",
    };
    let sinematik_stili = brif
        .map(|b| b.efektler.sinematik.as_str())
        .unwrap_or("kenburns+sinema");
    let cam_stili = brif
        .map(|b| b.efektler.cam.as_str())
        .unwrap_or("glassmorphism");
    let kaydirma_stili = brif
        .map(|b| b.efektler.kaydirma.as_str())
        .unwrap_or("scroll-parallax");
    let efekt = format!(
        " data-kenburns=\"{}\" data-glass=\"{}\" data-parallax=\"{}\" data-magnetic=\"{}\" data-imlec=\"{}\"",        if brif.is_none_or(|b| b.efektler.kenburns) {
            "1"
        } else {
            "0"
        },
        if brif.is_none_or(|b| b.efektler.glass) {
            "1"
        } else {
            "0"
        },
        if brif.is_none_or(|b| b.efektler.parallax) {
            "1"
        } else {
            "0"
        },
        if brif.is_none_or(|b| b.efektler.magnetic) {
            "1"
        } else {
            "0"
        },
        if brif.is_none_or(|b| b.efektler.imlec) {
            "1"
        } else {
            "0"
        }
    );

    let mut sayfa = String::new();
    sayfa.push_str(if tema_acik {
        "<!DOCTYPE html>\n<html lang=\"tr\" class=\"tema-acik\">\n<head>\n<meta charset=\"utf-8\">\n"
    } else {
        "<!DOCTYPE html>\n<html lang=\"tr\">\n<head>\n<meta charset=\"utf-8\">\n"
    });
    sayfa.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    sayfa.push_str(&format!("<title>{ad} — {sektor} | {bolge}</title>\n"));
    let aciklama = if adres.is_empty() {
        format!("{sektor} · {ad} · {}", ic.alt)
    } else {
        format!("{sektor} · {adres} · {}", ic.alt)
    };
    sayfa.push_str(&format!(
        concat!(
            "<meta name=\"description\" content=\"{aciklama}\">\n",
            "<meta property=\"og:type\" content=\"website\">\n",
            "<meta property=\"og:site_name\" content=\"{ad}\">\n",
            "<meta property=\"og:title\" content=\"{ad} — {sektor}\">\n",
            "<meta property=\"og:description\" content=\"{aciklama}\">\n",
            "<meta property=\"og:locale\" content=\"tr_TR\">\n",
            "<meta name=\"twitter:card\" content=\"summary\">\n",
            "<meta name=\"theme-color\" content=\"{vurgu}\">\n",
            "<meta name=\"robots\" content=\"noindex\">\n"
        ),
        aciklama = kacis(&aciklama),
        ad = ad,
        sektor = sektor,
        vurgu = vurgu
    ));
    sayfa.push_str("<style>\n");
    sayfa.push_str(g.font_css);
    sayfa.push_str(&format!(
        ":root{{--accent:{vurgu};--accent-2:{vurgu2};--accent-karsi:{};--bg:{zemin};--ink:{yazi};--r-1:10px;--r-2:18px;--r-3:26px;--r-4:38px;--aj-stagger:{stagger}ms;--f-baslik:{baslik};--f-govde:{govde}}}\n",
        if parlak_mi(vurgu) { "#12100e" } else { "#f8f5f0" },
        baslik = baslik_yigin,
        govde = govde_yigin,
        stagger = brif.map(|b| b.efektler.stagger_ms).unwrap_or(80)
    ));
    sayfa.push_str(CSS);
    sayfa.push_str(EK_CSS);
    sayfa.push_str(AJANS_CSS);
    sayfa.push_str("</style>\n<noscript><style>[data-aj-reveal]{opacity:1;transform:none;filter:none}.aj-kelime i{transform:none;opacity:1}.aj-imlec{display:none}</style></noscript>\n</head>\n");
    sayfa.push_str(&format!(
        r####"<body data-profil="ajans" data-stil="{stil}" data-hareket="{hareket}" data-yazi="{yazi_adi}" data-sinematik="{sinematik}" data-cam="{cam}" data-kaydirma="{kaydirma}"{kose}{efekt}>\n<div class="aj-glow" id="aj-glow" aria-hidden="true"></div><div class="aj-noise" aria-hidden="true"></div><div class="aj-kinetik" data-aj-par="0.04" aria-hidden="true"><div class="aj-kinetik-ic"><span>{ad}</span><span>{ad}</span></div></div>\n<div class="aj-ilerleme" id="aj-ilerleme" aria-hidden="true"></div>\n<div class="serit">Taslak demo · {ad} için hazırlandı · içerik taslaktır</div>\n"####,
        yazi_adi = if serif { "serif" } else { "grotesk" },
        sinematik = sinematik_stili,
        cam = cam_stili,
        kaydirma = kaydirma_stili,
        stil = stil
    ));

    sayfa.push_str(&format!(
        r####"<header class="ust" id="ust">\n<div class="ust-ic aj-ust-ic">\n<a class="marka" href="#ust"><span class="monogram">{harf}</span><span>{ad}</span></a>\n<nav class="menuler">\n<a href="#hakkimizda">Manifesto</a><a href="#hizmetler">Hizmetler</a><a href="#ai-asistan">Asistan</a><a href="#guven">Güven</a><a href="#isler">İşler</a><a href="#iletisim">İletişim</a>\n</nav>\n{tel}\n<button class="mobil-dugme" id="mobil-dugme" aria-expanded="false" aria-label="Menü">☰</button>\n</div>\n<nav class="mobil-menu" id="mobil-menu"><a href="#hakkimizda">Manifesto</a><a href="#hizmetler">Hizmetler</a><a href="#ai-asistan">Asistan</a><a href="#guven">Güven</a><a href="#isler">İşler</a><a href="#iletisim">İletişim</a></nav>\n</header>\n"####,
        harf = kacis(&harf),
        ad = ad,
        tel = match &tel_hedef {
            Some(hedef) => format!(
                r##"<a class="ust-tel" href="{hedef}">{TELEFON_IKON}{}</a>"##,
                kacis(&g.telefon_goster)
            ),
            None => String::new(),
        }
    ));

    sayfa.push_str(&format!(
        r####"<section class="aj-hero" id="hero"><div class="aj-diag" aria-hidden="true"><span>{sektor}</span><span>{sektor}</span></div>\n<div class="aj-hero-arka" data-aj-par="0.08" data-aj-pointer="1">{gorsel}<div class="aj-hero-perde"></div></div>\n<div class="aj-hero-ic kabuk">\n<div class="aj-hero-kopya">\n<span class="aj-kicker"><i></i>{etiket}</span>\n<h1 class="aj-manset"><span class="aj-satir">{h1}</span><span class="aj-satir">{h2}</span></h1>\n<div class="aj-cam aj-yuzen"><span>{bolge}</span><b>{sektor}</b></div>\n<p class="aj-hero-alt">{alt}</p>\n<div class="aj-hero-butonlar">{birincil}{ikincil}{konum}</div>\n<div class="aj-rozetler hero-rozetler">{rozetler}</div>\n</div>\n<aside class="aj-hero-yan"><div class="aj-cam aj-saat"><span>Yerel saat</span><b id="aj-saat">--:--</b><small>{sektor} · {bolge}</small></div>{puan}</aside>\n</div>\n<div class="aj-kaydir"><span>Kaydır</span><i></i></div>\n</section>\n<div class="aj-marquee" aria-hidden="true"><div class="aj-marquee-ic">{akis}{akis}</div></div>\n"####,
        gorsel = hero_gorsel,
        etiket = hero_etiket,
        h1 = h1,
        h2 = h2,
        alt = kacis(&hero_alt),
        birincil = birincil,
        ikincil = ikincil,
        konum = konum_buton,
        rozetler = hero_rozetler,
        sektor = sektor,
        bolge = bolge,
        puan = puan_kart,
        akis = akis
    ));

    sayfa.push_str(&format!(
        r####"<section class="aj-bolum aj-manifesto" id="hakkimizda" data-aj-section>\n<div class="kabuk aj-manifesto-ic">\n<div class="aj-bolum-yan"><span class="aj-no">01</span><span class="aj-dikey">{etiket}</span></div>\n<div class="aj-manifesto-metin"><span class="aj-eyebrow">{pusula}{sektor} · {bolge}</span><h2 data-aj-reveal>{baslik}</h2><p class="aj-buyuk" data-aj-reveal>{buyuk}</p><p data-aj-reveal>{ikinci}</p><a class="aj-metin-link" href="#hizmetler">Yaklaşımı gör <span>↘</span></a></div>\n<figure class="aj-manifesto-gorsel aj-cam" data-aj-reveal>{gorsel}<figcaption><span>{sektor}</span><span>{bolge}</span></figcaption></figure>\n</div>\n</section>\n"####,
        etiket = manifesto_etiket,
        pusula = ikon("pusula"),
        sektor = sektor,
        bolge = bolge,
        baslik = kacis(&manifesto_baslik),
        buyuk = kacis(&manifesto_buyuk),
        ikinci = kacis(&hikaye2),
        gorsel = manifesto_gorsel
    ));

    sayfa.push_str(&format!(
        r####"<section class="aj-bolum aj-hizmetler" id="hizmetler" data-aj-section>\n<div class="kabuk">\n<div class="aj-bolum-ust"><div class="aj-bolum-yan"><span class="aj-no">02</span><span class="aj-dikey">{etiket}</span></div><div><span class="aj-eyebrow">{kivilcim}{etiket}</span><h2 data-aj-reveal>{baslik}</h2><p data-aj-reveal>{alt}</p></div></div>\n<div class="aj-hizmet-grid">{kartlar}</div>\n</div>\n</section>\n"####,
        etiket = hizmet_etiket,
        kivilcim = ikon("kivilcim"),
        baslik = kacis(&hizmet_baslik),
        alt = kacis(&hizmet_alt),
        kartlar = hizmet_kartlari
    ));

    sayfa.push_str(&format!(
        r####"<section class="aj-bolum aj-ai" id="ai-asistan" data-aj-section>\n<div class="kabuk aj-ai-ic">\n<div class="aj-bolum-yan"><span class="aj-no">03</span><span class="aj-dikey">{etiket}</span></div>\n<div class="aj-ai-metin"><span class="aj-eyebrow">{bot}{etiket}</span><h2 data-aj-reveal>{baslik}</h2><p data-aj-reveal>{metin}</p><div class="aj-ai-kartlar">{kartlar}</div>{link}</div>\n<div class="aj-cam aj-ai-panel" data-aj-tilt data-aj-reveal><div class="aj-ai-panel-ust"><span>{bot}<i></i>Demo asistanı</span><b>7/24</b></div><p>Bir soru bırakın; doğru hizmeti ve sonraki adımı sade bir dille önersin.</p><div class="aj-ai-satir"><span>01</span>Dinler</div><div class="aj-ai-satir"><span>02</span>Yönlendirir</div><div class="aj-ai-satir"><span>03</span>Bağlar</div></div>\n</div>\n</section>\n"####,
        etiket = ai_etiket,
        bot = ikon("bot"),
        baslik = kacis(ai_baslik),
        metin = kacis(ai_metin),
        kartlar = ai_kartlar,
        link = ai_link
    ));

    sayfa.push_str(&guven_html(g, acil, &bolge, &cta, &wa_no));

    sayfa.push_str(&format!(
        r####"<section class="aj-bolum aj-isler" id="isler" data-aj-section>\n<div class="kabuk">\n<div class="aj-bolum-ust"><div class="aj-bolum-yan"><span class="aj-no">05</span><span class="aj-dikey">{etiket}</span></div><div><span class="aj-eyebrow">{etiket}</span><h2 data-aj-reveal>{baslik}</h2><p data-aj-reveal>{alt}</p></div></div>\n<div class="aj-is-grid">{kartlar}</div>\n</div>\n</section>\n"####,
        etiket = isler_etiket,
        baslik = kacis(isler_baslik),
        alt = kacis(isler_alt),
        kartlar = is_kartlari
    ));

    sayfa.push_str(&format!(
        r####"<section class="aj-bolum aj-iletisim" id="iletisim" data-aj-section>\n<div class="kabuk aj-iletisim-ic">\n<div class="aj-iletisim-sol"><div class="aj-bolum-yan"><span class="aj-no">06</span><span class="aj-dikey">{etiket}</span></div><span class="aj-eyebrow">{mesaj}{etiket}</span><h2 data-aj-reveal>{baslik}</h2><p data-aj-reveal>{metin}</p>{cta_link}<div class="aj-ilet-kartlar">{tel}{konum}</div>{saatler}</div>\n<div class="aj-cam aj-form-kart" data-aj-tilt><div class="aj-form-ust"><span>Kısa bir not</span><b>WhatsApp</b></div><form id="aj-form" data-wa="{wa}" novalidate><label>Adınız<input name="ad" type="text" autocomplete="name" required></label><label>Telefon<input name="tel" type="tel" inputmode="tel" autocomplete="tel" required></label><label>Mesajınız<textarea name="mesaj" rows="3" placeholder="Ne konuşmak istersiniz?"></textarea></label><button class="buton buton-dolu" type="submit">Gönder <span>↗</span></button><p class="aj-form-not" id="aj-form-not" role="status" aria-live="polite"></p></form>{harita}</div>\n</div>\n</section>\n"####,
        etiket = iletisim_etiket,
        mesaj = ikon("mesaj"),
        baslik = kacis(iletisim_baslik),
        metin = kacis(iletisim_metin),
        cta_link = match &tel_hedef {
            Some(hedef) => format!(
                r##"<a class="aj-buyuk-link" data-aj-magnetic href="{hedef}">{} <span>↗</span></a>"##,
                kacis(&cta)
            ),
            None => String::new(),
        },
        tel = tel_kart,
        konum = konum_kart,
        saatler = saatler,
        wa = wa_no,
        harita = harita
    ));

    sayfa.push_str(&format!(
        r####"<footer class="altbilgi aj-alt"><div class="kabuk alt-ic"><div><div class="alt-marka"><span class="monogram">{harf}</span>{ad}</div><p class="alt-not">Bu taslak {gonderen} tarafından {ad} için hazırlandı. Adres ve saat bilgileri {kaynak} kaydından alınmıştır.{foto_not}</p></div><div class="alt-baglar"><a href="#hakkimizda">Manifesto</a><a href="#hizmetler">Hizmetler</a><a href="#ai-asistan">Asistan</a><a href="#guven">Güven</a><a href="#isler">İşler</a><a href="#iletisim">İletişim</a>{karne}</div></div></footer>\n{mobil}\n{asistan}\n<div class="aj-imlec" id="aj-imlec" aria-hidden="true"></div>\n<div class="suresi-doldu" id="suresi-doldu" hidden><p>Bu demonun süresi doldu.<br>Güncel bir demo için işletmeyle iletişime geçin.</p></div>\n"####,
        harf = kacis(&harf),
        ad = ad,
        gonderen = kacis(&g.gonderen),
        kaynak = kacis(&g.kaynak),
        foto_not = foto_not,
        karne = karne,
        mobil = mobil,
        asistan = asistan
    ));

    sayfa.push_str(AJANS_JS);
    sayfa.push_str(&EK_JS.replace("__WA__", &wa_no));
    sayfa.push_str(&format!(
        concat!(
            "<script>\n",
            "(function(){{\n",
            "  var a='demo_{anahtar}', b=Number(localStorage.getItem(a)||Date.now());\n",
            "  localStorage.setItem(a,b);\n",
            "  var sure=document.getElementById('suresi-doldu');\n",
            "  if(sure && Date.now()-b>12*60*60*1000){{sure.hidden=false;document.body.style.overflow='hidden';}}\n",
            "}})();\n",
            "</script>\n</body>\n</html>\n"
        ),
        anahtar = g.anahtar
    ));
    sayfa
}

const AJANS_CSS: &str = r####"
body[data-profil=ajans]{--aj-max:1260px;--aj-ease:cubic-bezier(.19,.78,.18,1);background:var(--bg);color:var(--ink);overflow-x:hidden}
body[data-profil=ajans] .kabuk{max-width:var(--aj-max)}
body[data-profil=ajans] .ust{position:fixed;top:0;left:0;right:0;z-index:70;background:color-mix(in srgb,var(--bg) 58%,transparent);border-bottom:1px solid color-mix(in srgb,var(--ink) 10%,transparent);-webkit-backdrop-filter:blur(22px) saturate(1.3);backdrop-filter:blur(22px) saturate(1.3)}
body[data-profil=ajans] .ust-ic{padding:16px 0;gap:28px}
body[data-profil=ajans] .marka{letter-spacing:-.02em}
body[data-profil=ajans] .menuler{display:flex;gap:clamp(14px,2.4vw,30px);margin-left:auto;margin-right:auto;font-size:11px;letter-spacing:.12em;text-transform:uppercase}
body[data-profil=ajans] .menuler a{color:color-mix(in srgb,var(--ink) 62%,transparent);transition:color .25s}
body[data-profil=ajans] .menuler a:hover{color:var(--accent)}
body[data-profil=ajans] .ust-tel{font-size:11.5px;letter-spacing:.04em;border:1px solid color-mix(in srgb,var(--accent) 32%,transparent);border-radius:999px;padding:9px 14px;background:color-mix(in srgb,var(--accent) 8%,transparent);backdrop-filter:blur(12px)}
body[data-profil=ajans] .mobil-dugme{display:none}
body[data-profil=ajans] .mobil-menu{display:none}
body[data-profil=ajans] .mobil-menu.acik{display:flex;flex-direction:column;gap:4px;padding:10px 26px 18px;border-bottom:1px solid var(--cizgi);background:color-mix(in srgb,var(--bg) 94%,transparent)}
.aj-ilerleme{position:fixed;left:0;top:0;height:2px;width:0;background:linear-gradient(90deg,var(--accent),var(--accent-2));z-index:200;box-shadow:0 0 18px var(--accent)}
.aj-hero{position:relative;min-height:100svh;display:flex;align-items:flex-end;padding:clamp(130px,16vh,190px) 0 clamp(48px,7vw,90px);overflow:hidden;isolation:isolate}
.aj-hero-arka{position:absolute;inset:-4%;z-index:-3;will-change:transform}
.aj-hero-foto{width:100%;height:100%;object-fit:cover;object-position:50% 45%;animation:aj-kenburns 26s ease-in-out infinite alternate;filter:saturate(.92) contrast(1.06) brightness(.68)}
.aj-hero-perde{position:absolute;inset:0;background:linear-gradient(90deg,color-mix(in srgb,var(--bg) 94%,transparent) 0%,color-mix(in srgb,var(--bg) 58%,transparent) 46%,transparent 78%),linear-gradient(0deg,var(--bg) 0%,transparent 46%)}
.aj-doku{position:relative;width:100%;height:100%;min-height:260px;overflow:hidden;background:radial-gradient(80% 75% at 26% 20%,color-mix(in srgb,var(--accent) 46%,transparent),transparent 62%),radial-gradient(70% 70% at 80% 78%,color-mix(in srgb,var(--accent-2) 28%,transparent),transparent 64%),linear-gradient(145deg,color-mix(in srgb,var(--ink) 8%,var(--bg)),var(--bg))}
.aj-doku span{position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);font-family:var(--f-baslik);font-size:clamp(100px,22vw,280px);line-height:.75;color:transparent;-webkit-text-stroke:1px color-mix(in srgb,var(--accent) 50%,transparent);opacity:.5}
.aj-doku-hero{position:absolute;inset:0}
.aj-hero-ic{position:relative;z-index:2;display:grid;grid-template-columns:minmax(0,1.4fr) minmax(220px,.45fr);gap:clamp(28px,5vw,80px);align-items:end}
.aj-kicker,.aj-eyebrow{display:inline-flex;align-items:center;gap:10px;font-size:11px;font-weight:500;letter-spacing:.15em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 56%,transparent)}
.aj-kicker i{width:22px;height:1px;background:var(--accent)}
.aj-manset{font-family:var(--f-baslik);font-size:clamp(80px,10.5vw,190px);line-height:.86;letter-spacing:-.05em;font-weight:600;margin:20px 0 0;max-width:11ch;color:var(--ink);text-wrap:balance}
.aj-satir{display:block;overflow:hidden;padding-bottom:.06em}
.aj-kelime{display:inline-block;overflow:hidden;vertical-align:top}
.aj-kelime i{display:inline-block;font-style:inherit;transform:translateY(112%);opacity:0;transition:transform 1.1s var(--aj-ease),opacity .8s ease;transition-delay:calc(var(--i,0) * var(--aj-stagger,80ms))}
body.aj-hazir .aj-kelime i{transform:none;opacity:1}
.aj-hero-alt{max-width:46ch;margin:24px 0 0;font-size:clamp(14px,1.25vw,17px);line-height:1.7;color:color-mix(in srgb,var(--ink) 68%,transparent)}
.aj-hero-butonlar{display:flex;flex-wrap:wrap;align-items:center;gap:12px;margin-top:28px}
.aj-buton{display:inline-flex;align-items:center;gap:10px;padding:15px 20px;border-radius:999px;background:var(--accent);color:var(--accent-karsi);font-weight:750;font-size:12.5px;letter-spacing:.02em;box-shadow:0 18px 50px -22px var(--accent)}
.aj-buton b{font-size:15px;margin-left:2px}
.aj-metin-link{display:inline-flex;align-items:center;gap:7px;font-size:11.5px;letter-spacing:.12em;text-transform:uppercase;border-bottom:1px solid color-mix(in srgb,var(--accent) 55%,transparent);padding-bottom:5px;color:color-mix(in srgb,var(--ink) 78%,transparent)}
.aj-metin-link span{color:var(--accent);font-size:15px}
.aj-rozetler{display:flex;flex-wrap:wrap;gap:8px;margin-top:26px}
.aj-rozet{display:inline-flex;align-items:center;gap:8px;padding:9px 13px;border:1px solid color-mix(in srgb,var(--accent) 26%,transparent);border-radius:999px;background:color-mix(in srgb,var(--bg) 54%,transparent);-webkit-backdrop-filter:blur(16px) saturate(1.2);backdrop-filter:blur(16px) saturate(1.2);font-size:10.5px;letter-spacing:.04em}
.aj-rozet .aj-ikon{width:13px;height:13px;color:var(--accent-2)}
.aj-rozet.canli .aj-ikon{color:#4ade80;animation:nabiz 2s infinite}
.aj-hero-yan{display:flex;flex-direction:column;gap:12px;align-items:stretch}
.aj-cam{background:linear-gradient(145deg,color-mix(in srgb,var(--bg) 76%,transparent),color-mix(in srgb,var(--bg) 40%,transparent));border:1px solid rgba(255,255,255,.08);-webkit-backdrop-filter:blur(24px) saturate(1.35);backdrop-filter:blur(24px) saturate(1.35);box-shadow:0 24px 70px -34px color-mix(in srgb,var(--bg) 80%,#000),inset 0 1px 0 color-mix(in srgb,var(--ink) 10%,transparent);border-radius:var(--r-3)}
body[data-glass="0"] .aj-cam{-webkit-backdrop-filter:none;backdrop-filter:none;background:color-mix(in srgb,var(--bg) 92%,transparent)}
.aj-saat{padding:18px 18px 15px;display:flex;flex-direction:column;gap:4px}
.aj-saat span,.aj-saat small{font-size:9px;letter-spacing:.2em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 50%,transparent)}
.aj-saat b{font-family:var(--f-baslik);font-size:clamp(28px,3vw,44px);line-height:1;letter-spacing:-.03em;font-weight:500}
.aj-puan{padding:16px 18px;display:flex;flex-direction:column;gap:4px}
.aj-yildiz{display:flex;gap:2px;color:var(--accent-2)}
.aj-yildiz svg{width:12px;height:12px}
.aj-puan b{font-size:22px;font-family:var(--f-baslik);font-weight:500}
.aj-puan span{font-size:10px;color:color-mix(in srgb,var(--ink) 55%,transparent)}
.aj-kaydir{position:absolute;right:clamp(18px,4vw,46px);bottom:clamp(20px,3vw,38px);z-index:2;display:flex;align-items:center;gap:10px;writing-mode:vertical-rl;font-size:9px;letter-spacing:.3em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 46%,transparent)}
.aj-kaydir i{display:block;width:1px;height:52px;background:linear-gradient(var(--accent),transparent);animation:ajRay 2.4s ease-in-out infinite}
.aj-marquee{overflow:hidden;border-top:1px solid color-mix(in srgb,var(--ink) 10%,transparent);border-bottom:1px solid color-mix(in srgb,var(--ink) 10%,transparent);padding:13px 0;background:color-mix(in srgb,var(--accent) 5%,transparent)}
.aj-marquee-ic{display:flex;width:max-content;gap:38px;animation:ajMarquee 38s linear infinite;font-size:10px;letter-spacing:.28em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 52%,transparent)}
.aj-marquee-ic span{white-space:nowrap}
.aj-marquee-ic span::before{content:"✦";color:var(--accent);margin-right:38px}
.aj-bolum{position:relative;padding:clamp(128px,12.5vw,196px) 0;border-bottom:0}
.aj-bolum::after{content:"";position:absolute;left:0;right:0;bottom:0;height:1px;background:linear-gradient(90deg,transparent,color-mix(in srgb,var(--accent) 48%,transparent) 16%,color-mix(in srgb,var(--ink) 14%,transparent) 54%,transparent 94%);opacity:.9}
.aj-bolum-yan{display:flex;align-items:flex-start;gap:12px}
.aj-no{font-family:var(--f-baslik);font-size:clamp(28px,3vw,48px);line-height:.8;letter-spacing:-.04em;color:var(--accent)}
.aj-dikey{writing-mode:vertical-rl;font-size:9px;letter-spacing:.3em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 45%,transparent)}
.aj-manifesto-ic{display:grid;grid-template-columns:52px minmax(0,1.05fr) minmax(260px,.7fr);gap:clamp(22px,4vw,56px);align-items:start}
.aj-manifesto-metin h2,.aj-bolum-ust h2,.aj-iletisim-sol h2{font-family:var(--f-baslik);font-size:clamp(36px,5.8vw,86px);line-height:.92;letter-spacing:-.045em;font-weight:600;margin:16px 0 0;max-width:13ch}
.aj-buyuk{font-size:clamp(17px,1.6vw,22px);line-height:1.55;margin:24px 0 0;color:color-mix(in srgb,var(--ink) 84%,transparent);max-width:44ch}
.aj-manifesto-metin p:not(.aj-buyuk){font-size:14px;line-height:1.8;color:color-mix(in srgb,var(--ink) 62%,transparent);max-width:48ch}
.aj-manifesto-metin .aj-metin-link{margin-top:24px}
.aj-manifesto-gorsel{position:relative;overflow:hidden;min-height:clamp(300px,42vw,520px);padding:0;margin:0}
.aj-manifesto-gorsel img{width:100%;height:100%;min-height:inherit;object-fit:cover;animation:aj-kenburns 30s ease-in-out infinite alternate}
.aj-manifesto-gorsel figcaption{position:absolute;left:14px;right:14px;bottom:14px;display:flex;justify-content:space-between;font-size:9px;letter-spacing:.2em;text-transform:uppercase;color:rgba(255,255,255,.72);text-shadow:0 1px 10px rgba(0,0,0,.5)}
.aj-bolum-ust{display:grid;grid-template-columns:52px minmax(0,1fr);gap:clamp(22px,4vw,56px);margin-bottom:clamp(30px,5vw,64px)}
.aj-bolum-ust p{max-width:52ch;margin:16px 0 0;font-size:14px;line-height:1.75;color:color-mix(in srgb,var(--ink) 62%,transparent)}
.aj-hizmet-grid{display:grid;grid-template-columns:repeat(12,minmax(0,1fr));grid-auto-rows:minmax(104px,auto);grid-auto-flow:dense;gap:12px}
.aj-hizmet:nth-child(1){grid-column:span 8;grid-row:span 2;min-height:clamp(320px,36vw,470px)}
.aj-hizmet:nth-child(2){grid-column:span 4;grid-row:span 2;justify-content:space-between;background:linear-gradient(180deg,color-mix(in srgb,var(--accent) 12%,transparent),transparent 74%)}
.aj-hizmet:nth-child(2) h3{margin-top:auto;font-size:clamp(20px,1.9vw,27px)}
.aj-hizmet:nth-child(3){grid-column:span 5}
.aj-hizmet:nth-child(4){grid-column:span 7;min-height:0;padding:clamp(18px,2.2vw,26px) clamp(22px,3vw,34px);flex-direction:row;align-items:center;gap:clamp(16px,3vw,40px)}
.aj-hizmet:nth-child(4) .aj-hizmet-ust{order:2;margin-left:auto}
.aj-hizmet:nth-child(4) h3{margin:0;font-size:clamp(19px,2vw,27px);white-space:nowrap}
.aj-hizmet:nth-child(4) p{margin:0;max-width:46ch}
.aj-hizmet:nth-child(4) .aj-hizmet-ok{position:static;order:3;margin-left:4px;font-size:15px}
.aj-mikro-bar{grid-column:span 12;display:grid;grid-template-columns:repeat(3,minmax(0,1fr));margin-top:2px;border-top:1px solid color-mix(in srgb,var(--ink) 12%,transparent);border-bottom:1px solid color-mix(in srgb,var(--ink) 12%,transparent);background:linear-gradient(90deg,color-mix(in srgb,var(--accent) 6%,transparent),transparent 62%)}
.aj-mikro{display:flex;align-items:baseline;gap:10px;min-width:0;padding:15px 0 15px clamp(2px,1vw,14px)}
.aj-mikro + .aj-mikro{border-left:1px solid color-mix(in srgb,var(--ink) 10%,transparent);padding-left:clamp(12px,2vw,24px)}
.aj-mikro span{flex:0 0 auto;font-size:8.5px;letter-spacing:.24em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 44%,transparent)}
.aj-mikro b{min-width:0;font-family:var(--f-baslik);font-size:13px;font-weight:500;letter-spacing:-.01em;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.aj-hizmet{position:relative;padding:clamp(22px,3vw,34px);min-height:220px;display:flex;flex-direction:column;overflow:hidden;transform:perspective(900px) rotateX(var(--rx,0deg)) rotateY(var(--ry,0deg));transition:transform .4s var(--aj-ease),border-color .4s,box-shadow .45s var(--aj-ease)}
.aj-hizmet:nth-child(1){background:radial-gradient(120% 120% at 10% 0%,color-mix(in srgb,var(--accent) 26%,transparent),transparent 62%),linear-gradient(150deg,color-mix(in srgb,var(--bg) 84%,transparent),color-mix(in srgb,var(--bg) 46%,transparent))}
.aj-hizmet:nth-child(1) h3{font-size:clamp(28px,3.6vw,50px);max-width:14ch;margin-top:auto;line-height:1}
.aj-hizmet:nth-child(1) p{max-width:40ch}
.aj-hizmet:hover{border-color:color-mix(in srgb,var(--accent) 56%,transparent);box-shadow:0 44px 110px -56px var(--accent),0 0 0 1px color-mix(in srgb,var(--accent) 34%,transparent),inset 0 1px 0 rgba(255,255,255,.16);transform:perspective(1100px) rotateX(var(--rx,0deg)) rotateY(var(--ry,0deg)) translateY(-8px)}
.aj-ikon{width:15px;height:15px;flex:0 0 auto;stroke-width:1.7}
.aj-eyebrow .aj-ikon{width:13px;height:13px;color:var(--accent)}
.aj-hizmet-ust{display:flex;align-items:center;justify-content:space-between;gap:10px}
.aj-hizmet-ikon{display:inline-flex;color:var(--accent);opacity:.7;transition:opacity .35s,transform .35s}
.aj-hizmet-ikon .aj-ikon{width:20px;height:20px;stroke-width:1.5}
.aj-hizmet:hover .aj-hizmet-ikon{opacity:1;transform:scale(1.04)}
.aj-hizmet:hover .aj-hizmet-ikon .aj-ikon{transform:scale(1.04)}
.aj-hizmet-no{font-size:10px;letter-spacing:.2em;color:var(--accent-2)}
.aj-hizmet h3{font-family:var(--f-baslik);font-size:clamp(23px,2.6vw,34px);line-height:1.04;letter-spacing:-.03em;margin:24px 0 0;font-weight:600}
.aj-hizmet p{margin:10px 0 0;font-size:13px;line-height:1.7;color:color-mix(in srgb,var(--ink) 60%,transparent);max-width:34ch}
.aj-hizmet-ok{position:absolute;right:20px;top:18px;color:var(--accent);font-size:18px;opacity:.45;transition:transform .35s,opacity .35s}
.aj-hizmet:hover .aj-hizmet-ok{transform:translate(3px,-3px);opacity:1}
.aj-is-grid{display:grid;grid-template-columns:repeat(12,minmax(0,1fr));grid-auto-rows:minmax(120px,auto);grid-auto-flow:dense;gap:12px;align-items:stretch}
.aj-is:nth-child(1){grid-column:span 7;grid-row:span 2;min-height:clamp(380px,44vw,560px)}
.aj-is:nth-child(2),.aj-is:nth-child(3){grid-column:span 5;min-height:clamp(170px,20vw,250px)}
.aj-is:hover{border-color:color-mix(in srgb,var(--accent) 44%,transparent);box-shadow:0 30px 80px -50px var(--accent)}
.aj-is{position:relative;margin:0;min-height:clamp(240px,32vw,420px);overflow:hidden;border-radius:var(--r-3);border:1px solid color-mix(in srgb,var(--ink) 12%,transparent);background:color-mix(in srgb,var(--ink) 4%,var(--bg))}
.aj-is img{width:100%;height:100%;object-fit:cover;transition:transform 1s var(--aj-ease),filter .6s;animation:aj-kenburns 30s ease-in-out infinite alternate}
.aj-is:hover img{transform:scale(1.04);filter:saturate(1.08)}
.aj-is{transition:transform .55s var(--aj-ease),border-color .45s,box-shadow .45s}
.aj-is:hover{transform:translateY(-6px)}
.aj-is figcaption{position:absolute;left:16px;right:16px;bottom:16px;display:flex;flex-direction:column;gap:4px;padding:14px;border-radius:var(--r-2);background:color-mix(in srgb,var(--bg) 66%,transparent);border:1px solid color-mix(in srgb,var(--ink) 12%,transparent);-webkit-backdrop-filter:blur(18px) saturate(1.2);backdrop-filter:blur(18px) saturate(1.2)}
.aj-is figcaption span{font-size:9px;letter-spacing:.22em;color:var(--accent-2)}
.aj-is figcaption b{font-size:13px;line-height:1.3}
.aj-is figcaption small{font-size:10.5px;line-height:1.5;color:color-mix(in srgb,var(--ink) 58%,transparent);display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.aj-is-doku .aj-doku{position:absolute;inset:0}
.aj-is-doku figcaption{background:color-mix(in srgb,var(--bg) 78%,transparent)}
.aj-ai{background:radial-gradient(100% 90% at 18% 8%,color-mix(in srgb,var(--accent) 12%,transparent),transparent 58%)}
.aj-ai-ic{display:grid;grid-template-columns:52px minmax(0,1.05fr) minmax(280px,.7fr);gap:clamp(22px,4vw,56px);align-items:start}
.aj-ai-ic .aj-bolum-yan{grid-column:1;grid-row:1}
.aj-ai-metin{grid-column:2;grid-row:1}
.aj-ai-metin h2{font-family:var(--f-baslik);font-size:clamp(32px,5vw,70px);line-height:.98;letter-spacing:-.04em;font-weight:600;margin:14px 0 0;max-width:12ch}
.aj-ai-metin > p:not(.aj-eyebrow){max-width:48ch;margin:18px 0 0;font-size:14px;line-height:1.8;color:color-mix(in srgb,var(--ink) 64%,transparent)}
.aj-ai-kartlar{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px;margin-top:24px;max-width:560px}
.aj-ai-kart{padding:13px 14px;border:1px solid color-mix(in srgb,var(--ink) 12%,transparent);border-radius:var(--r-2);background:color-mix(in srgb,var(--bg) 46%,transparent);backdrop-filter:blur(14px)}
.aj-ai-kart b{display:block;font-size:12px;line-height:1.35;font-weight:700}
.aj-ai-kart span{display:block;margin-top:4px;font-size:10.5px;line-height:1.55;color:color-mix(in srgb,var(--ink) 56%,transparent)}
.aj-ai-metin .aj-buyuk-link{margin-top:24px;font-size:clamp(24px,3vw,38px)}
.aj-ai-panel{grid-column:3;grid-row:1;padding:clamp(18px,2.4vw,26px);min-height:330px;display:flex;flex-direction:column;justify-content:space-between;overflow:hidden}
.aj-ai-panel-ust{display:flex;align-items:center;justify-content:space-between;gap:12px}
.aj-ai-panel-ust span{display:inline-flex;align-items:center;gap:7px;font-size:9px;letter-spacing:.18em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 58%,transparent)}
.aj-ai-panel-ust i{width:6px;height:6px;border-radius:50%;background:#4ade80;box-shadow:0 0 0 0 rgba(74,222,128,.55);animation:nabiz 2s infinite}
.aj-ai-panel-ust b{font-family:var(--f-baslik);font-size:25px;font-weight:500;color:var(--accent)}
.aj-ai-panel-ust .aj-ikon{width:15px;height:15px;color:var(--accent)}
.aj-ai-panel > p{margin:16px 0 0;font-size:12.5px;line-height:1.7;color:color-mix(in srgb,var(--ink) 62%,transparent)}
.aj-ai-satir{display:flex;align-items:center;gap:10px;margin-top:9px;padding-top:9px;border-top:1px solid color-mix(in srgb,var(--ink) 10%,transparent);font-size:11px;letter-spacing:.04em;color:color-mix(in srgb,var(--ink) 72%,transparent)}
.aj-ai-satir span{font-size:9px;letter-spacing:.16em;color:var(--accent-2)}
.aj-iletisim{background:radial-gradient(120% 100% at 80% 0%,color-mix(in srgb,var(--accent) 18%,transparent),transparent 60%)}
.aj-iletisim-ic{display:grid;grid-template-columns:minmax(0,1fr) minmax(300px,.72fr);gap:clamp(28px,5vw,72px);align-items:start}
.aj-iletisim-sol{display:flex;flex-direction:column;align-items:flex-start}
.aj-iletisim-sol .aj-bolum-yan{margin-bottom:22px}
.aj-iletisim-sol p:not(.aj-eyebrow){max-width:46ch;margin:18px 0 0;font-size:14px;line-height:1.8;color:color-mix(in srgb,var(--ink) 64%,transparent)}
.aj-buyuk-link{display:inline-flex;align-items:center;gap:14px;margin-top:28px;font-family:var(--f-baslik);font-size:clamp(28px,4.4vw,58px);line-height:1;letter-spacing:-.04em;border-bottom:1px solid color-mix(in srgb,var(--accent) 48%,transparent);padding-bottom:9px}
.aj-buyuk-link span{font-size:.55em;color:var(--accent)}
.aj-ilet-kartlar{display:flex;flex-wrap:wrap;gap:10px;margin-top:30px}
.aj-ilet-kart{display:flex;align-items:flex-start;gap:10px;padding:13px 15px;border:1px solid var(--cizgi);border-radius:var(--r-2);background:color-mix(in srgb,var(--bg) 50%,transparent);backdrop-filter:blur(14px);max-width:100%}
.aj-ilet-kart svg{width:16px;height:16px;color:var(--accent);flex:0 0 auto;margin-top:2px}
.aj-ilet-kart small{display:block;font-size:8.5px;letter-spacing:.18em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 46%,transparent);margin-bottom:3px}
.aj-ilet-kart b{font-size:12px;line-height:1.5;font-weight:600}
.aj-iletisim .saatlar{margin-top:26px;width:100%;max-width:430px;border-color:color-mix(in srgb,var(--ink) 10%,transparent);background:color-mix(in srgb,var(--bg) 36%,transparent)}
.aj-form-kart{padding:clamp(18px,2.4vw,26px);display:flex;flex-direction:column;gap:14px}
.aj-form-ust{display:flex;align-items:center;justify-content:space-between;font-size:9px;letter-spacing:.2em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 52%,transparent)}
.aj-form-ust b{color:var(--accent-2);font-weight:600}
.aj-form-kart form{display:flex;flex-direction:column;gap:10px}
.aj-form-kart label{display:flex;flex-direction:column;gap:5px;font-size:9px;letter-spacing:.12em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 55%,transparent)}
.aj-form-kart input,.aj-form-kart textarea{width:100%;border:1px solid color-mix(in srgb,var(--ink) 14%,transparent);border-radius:12px;background:color-mix(in srgb,var(--bg) 46%,transparent);color:var(--ink);padding:12px 13px;font:inherit;font-size:13px;outline:none;resize:vertical}
.aj-form-kart input:focus,.aj-form-kart textarea:focus{border-color:var(--accent);box-shadow:0 0 0 3px color-mix(in srgb,var(--accent) 14%,transparent)}
.aj-form-kart .buton{width:100%;justify-content:space-between;margin-top:4px;border-radius:12px}
.aj-form-not{min-height:15px;margin:0;font-size:10.5px;line-height:1.5;color:color-mix(in srgb,var(--ink) 54%,transparent)}
.aj-form-kart .harita{margin-top:12px;border-radius:var(--r-2)}
.aj-form-kart .harita iframe{height:190px}
.aj-alt{padding:38px 0 96px;border-top:1px solid color-mix(in srgb,var(--ink) 10%,transparent)}
.aj-alt .alt-ic{align-items:flex-start}
.aj-alt .alt-not{max-width:64ch}
[data-aj-reveal]{opacity:0;transform:translateY(34px);filter:blur(12px);transition:opacity 1s cubic-bezier(.16,1,.3,1),transform 1s cubic-bezier(.16,1,.3,1),filter 1s cubic-bezier(.16,1,.3,1);transition-delay:calc(var(--i,0) * var(--aj-stagger,80ms));will-change:opacity,transform,filter}
[data-aj-reveal].gorunur{opacity:1;transform:none;filter:blur(0)}
.aj-imlec{position:fixed;left:0;top:0;width:8px;height:8px;border-radius:50%;background:var(--accent);z-index:300;pointer-events:none;transform:translate(-50%,-50%);mix-blend-mode:difference;transition:width .2s,height .2s,opacity .2s;opacity:0}
.aj-imlec::after{content:"";position:absolute;inset:-9px;border:1px solid color-mix(in srgb,var(--accent) 46%,transparent);border-radius:50%;transition:transform .25s}
body[data-imlec="1"] .aj-imlec{opacity:1}
body[data-imlec="1"] .aj-imlec.buyuk{width:34px;height:34px;background:transparent;border:1px solid var(--accent)}
body[data-imlec="1"] .aj-imlec.buyuk::after{transform:scale(.6)}
body[data-magnetic="0"] .aj-buton,body[data-magnetic="0"] .aj-buyuk-link{transform:none!important}
body[data-kenburns="0"] .aj-hero-foto,body[data-kenburns="0"] .aj-manifesto-gorsel img,body[data-kenburns="0"] .aj-is img{animation:none}
@keyframes aj-kenburns{0%{transform:scale(1.02) translate3d(0,0,0)}55%{transform:scale(1.11) translate3d(-1.2%,-1%,0)}100%{transform:scale(1.18) translate3d(1.4%,-1.8%,0)}}
@keyframes ajMarquee{from{transform:translateX(0)}to{transform:translateX(-50%)}}
@keyframes ajRay{0%,100%{transform:scaleY(.35);transform-origin:top;opacity:.4}50%{transform:scaleY(1);opacity:1}}
/* Açık yerleşim: otomatik yerleştirme, dekoratif bir katman nedeniyle
   bazı tarayıcılarda ilk sütunu atlayabiliyor; sütunları sabitliyoruz. */
.aj-hero-kopya{grid-column:1;grid-row:1}
.aj-hero-yan{grid-column:2;grid-row:1;align-self:end}
.aj-manifesto-ic .aj-bolum-yan{grid-column:1;grid-row:1}
.aj-manifesto-ic .aj-manifesto-metin{grid-column:2;grid-row:1}
.aj-manifesto-ic .aj-manifesto-gorsel{grid-column:3;grid-row:1}
.aj-bolum-ust .aj-bolum-yan{grid-column:1;grid-row:1}
.aj-bolum-ust > div:last-child{grid-column:2;grid-row:1}
.aj-iletisim-sol{grid-column:1;grid-row:1}
.aj-iletisim .aj-form-kart{grid-column:2;grid-row:1}
@media (max-width:1000px){
 body[data-profil=ajans] .menuler,body[data-profil=ajans] .ust-tel{display:none}
 body[data-profil=ajans] .mobil-dugme{display:inline-flex}
 body[data-profil=ajans] .mobil-menu{position:fixed;top:68px;left:0;right:0;z-index:69}
 .aj-hero{padding-top:112px}
 .aj-hero-ic{grid-template-columns:1fr;gap:26px}
 .aj-hero-kopya,.aj-hero-yan{grid-column:1;grid-row:auto}
 .aj-hero-yan{flex-direction:row;flex-wrap:wrap}
 .aj-saat{flex:1;min-width:160px}
 .aj-puan{flex:1;min-width:160px}
 .aj-manset{font-size:clamp(42px,11.5vw,84px);max-width:none}
 .aj-manifesto-ic{grid-template-columns:36px 1fr}
 .aj-manifesto-ic .aj-bolum-yan{grid-column:1;grid-row:1}
 .aj-manifesto-ic .aj-manifesto-metin{grid-column:2;grid-row:1}
 .aj-manifesto-gorsel{grid-column:1/-1;grid-row:2;min-height:300px}
 .aj-bolum-ust{grid-template-columns:36px 1fr}
 .aj-bolum-ust .aj-bolum-yan{grid-column:1;grid-row:1}
 .aj-bolum-ust > div:last-child{grid-column:2;grid-row:1}
 .aj-hizmet-grid{grid-template-columns:1fr;grid-auto-rows:auto}
 .aj-hizmet:nth-child(n){grid-column:auto;grid-row:auto;min-height:200px;flex-direction:column;align-items:stretch}
 .aj-hizmet:nth-child(1){min-height:300px}
 .aj-hizmet:nth-child(4) .aj-hizmet-ust{order:0;margin-left:0}
 .aj-hizmet:nth-child(4) h3{white-space:normal}
 .aj-hizmet:nth-child(4) .aj-hizmet-ok{position:absolute;order:0}
 .aj-mikro-bar{grid-column:1;grid-template-columns:1fr}
 .aj-mikro + .aj-mikro{border-left:0;border-top:1px solid color-mix(in srgb,var(--ink) 10%,transparent);padding-left:2px}
 .aj-is-grid{grid-template-columns:1fr 1fr}
 .aj-is:first-child{grid-column:1/-1;min-height:320px}
 .aj-ai-ic{grid-template-columns:36px 1fr}
 .aj-ai-ic .aj-bolum-yan{grid-column:1;grid-row:1}
 .aj-ai-metin{grid-column:2;grid-row:1}
 .aj-ai-panel{grid-column:2;grid-row:2;min-height:220px}
 .aj-iletisim-ic{grid-template-columns:1fr}
 .aj-iletisim-sol,.aj-iletisim .aj-form-kart{grid-column:1;grid-row:auto}
 .aj-imlec{display:none}
}
@media (max-width:560px){
 .aj-is-grid{grid-template-columns:1fr}
 .aj-is:first-child{min-height:300px}
 .aj-manset{font-size:clamp(60px,16vw,96px)}
 .aj-hero-alt{font-size:14px}
 .aj-hero-butonlar{gap:9px}
 .aj-bolum{padding:88px 0}
}
@media (prefers-reduced-motion:reduce){
 .aj-kelime i{transform:none;opacity:1}
 [data-aj-reveal]{opacity:1;transform:none;filter:none}
 .aj-hero-foto,.aj-manifesto-gorsel img,.aj-is img,.aj-marquee-ic,.aj-kaydir i{animation:none}
 .aj-hizmet{transform:none}
 .aj-glow{display:none}
 .aj-kinetik,.aj-diag{display:none}
 .aj-hero-arka{transform:none;transition:none}
 .aj-cam[data-aj-tilt]{transform:none}
 .aj-buton{animation:none}
}
/* ── 3B imleç ışığı: cam yüzeylerin arkasında gezinen huzme ─────────── */
.aj-glow{position:fixed;inset:0;z-index:0;pointer-events:none;background:radial-gradient(26vmax 26vmax at var(--aj-mx,50vw) var(--aj-my,30vh),color-mix(in srgb,var(--accent) 20%,transparent),transparent 62%),radial-gradient(54vmax 54vmax at var(--aj-mx,50vw) var(--aj-my,30vh),color-mix(in srgb,var(--accent-2) 9%,transparent),transparent 72%);opacity:0;transition:opacity .9s ease;mix-blend-mode:screen}
.aj-noise{position:fixed;inset:0;z-index:0;pointer-events:none;opacity:.02;mix-blend-mode:overlay;background-image:url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.85' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='180' height='180' filter='url(%23n)' opacity='.55'/%3E%3C/svg%3E");background-size:180px 180px}
body[data-stil="temiz-cam"] .aj-noise{opacity:.015;mix-blend-mode:multiply}
body.aj-isik .aj-glow{opacity:.9}
body[data-glass="0"] .aj-glow{opacity:.45}
body[data-profil=ajans] > section,.aj-marquee,.aj-alt{position:relative;z-index:1}
.aj-hero-arka{transform:translate3d(var(--aj-px,0px),calc(var(--aj-par,0px) * .5 + var(--aj-py,0px)),0);transition:transform .5s var(--aj-ease)}
.aj-cam[data-aj-tilt]{position:relative;overflow:hidden;transform:perspective(1100px) rotateX(var(--rx,0deg)) rotateY(var(--ry,0deg));transition:transform .45s var(--aj-ease),border-color .4s}
.aj-cam[data-aj-tilt]::before{content:"";position:absolute;inset:0;pointer-events:none;opacity:0;transition:opacity .45s;mix-blend-mode:screen;background:radial-gradient(280px 280px at var(--aj-kx,50%) var(--aj-ky,50%),color-mix(in srgb,var(--accent) 34%,transparent),transparent 70%)}
.aj-cam[data-aj-tilt]:hover::before{opacity:1}
/* ── Güven ve dönüşüm bölümü ────────────────────────────────────────── */
.aj-guven{background:radial-gradient(90% 70% at 82% 0%,color-mix(in srgb,var(--accent) 12%,transparent),transparent 58%)}
.aj-sayaclar{display:grid;grid-template-columns:repeat(12,minmax(0,1fr));gap:1px;margin:0 0 14px;border:1px solid color-mix(in srgb,var(--ink) 14%,transparent);border-radius:var(--r-3);overflow:hidden;background:color-mix(in srgb,var(--ink) 14%,transparent)}
.aj-sayac:nth-child(1){grid-column:span 5}
.aj-sayac:nth-child(2){grid-column:span 3}
.aj-sayac:nth-child(3){grid-column:span 2}
.aj-sayac:nth-child(4){grid-column:span 2}
.aj-sayac{display:flex;flex-direction:column;gap:6px;padding:clamp(16px,2.2vw,26px);background:color-mix(in srgb,var(--bg) 90%,transparent)}
.aj-sayac b{display:flex;align-items:baseline;gap:2px;font-family:var(--f-baslik);font-size:clamp(30px,4.2vw,54px);line-height:1;letter-spacing:-.045em;font-weight:600}
.aj-sayac b i{font-style:normal;font-size:.38em;letter-spacing:0;color:var(--accent);transform:translateY(-.18em)}
.aj-sayac span{font-size:9.5px;letter-spacing:.16em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 52%,transparent)}
.aj-guven-rozetler{display:flex;flex-wrap:wrap;gap:8px;margin:0 0 clamp(30px,4vw,48px)}
.aj-guven-rozet{display:inline-flex;align-items:center;gap:8px;padding:10px 14px;border-radius:999px;border:1px solid color-mix(in srgb,var(--accent) 26%,transparent);background:color-mix(in srgb,var(--bg) 54%,transparent);-webkit-backdrop-filter:blur(16px) saturate(1.2);backdrop-filter:blur(16px) saturate(1.2);font-size:10.5px;letter-spacing:.05em;white-space:nowrap}
.aj-guven-rozet .aj-ikon{width:14px;height:14px;color:var(--accent-2)}
.aj-kademe-baslik,.aj-sss-baslik{display:flex;flex-wrap:wrap;align-items:baseline;justify-content:space-between;gap:10px;margin:0 0 14px}
.aj-kademe-baslik p{margin:0;font-size:12px;color:color-mix(in srgb,var(--ink) 54%,transparent)}
.aj-sss-baslik h3{margin:0;font-family:var(--f-baslik);font-size:clamp(22px,2.6vw,32px);font-weight:600;letter-spacing:-.03em}
.aj-kademeler{display:grid;grid-template-columns:repeat(12,minmax(0,1fr));gap:12px;margin:0 0 clamp(34px,4.4vw,56px)}
.aj-kademe:nth-child(1){grid-column:span 5}
.aj-kademe:nth-child(2){grid-column:span 4}
.aj-kademe:nth-child(3){grid-column:span 3}
.aj-kademe{display:flex;flex-direction:column;gap:12px;padding:clamp(18px,2.4vw,28px);min-width:0}
.aj-kademe-one{border-color:color-mix(in srgb,var(--accent) 52%,transparent);background:linear-gradient(160deg,color-mix(in srgb,var(--accent) 16%,transparent),color-mix(in srgb,var(--bg) 52%,transparent))}
.aj-kademe-ust{display:flex;align-items:center;justify-content:space-between;gap:8px;font-size:9px;letter-spacing:.2em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 52%,transparent)}
.aj-kademe-ust b{padding:3px 7px;border:1px solid color-mix(in srgb,var(--accent) 40%,transparent);border-radius:999px;font-size:8.5px;font-weight:700;letter-spacing:.08em;color:var(--accent)}
.aj-kademe h3{margin:2px 0 0;font-family:var(--f-baslik);font-size:clamp(21px,2.2vw,28px);font-weight:600;line-height:1.1;letter-spacing:-.02em}
.aj-kademe p{margin:0;font-size:12px;line-height:1.65;color:color-mix(in srgb,var(--ink) 60%,transparent)}
.aj-kademe ul{display:flex;flex-direction:column;gap:8px;margin:6px 0 0;padding:0;list-style:none}
.aj-kademe li{display:flex;gap:8px;font-size:11.5px;line-height:1.5;color:color-mix(in srgb,var(--ink) 74%,transparent)}
.aj-kademe li .aj-ikon{width:13px;height:13px;margin-top:2px;color:var(--accent)}
.aj-kademe-cta{display:inline-flex;align-items:center;justify-content:space-between;gap:8px;margin-top:auto;padding-top:14px;border-top:1px solid color-mix(in srgb,var(--ink) 12%,transparent);font-size:11px;letter-spacing:.12em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 82%,transparent)}
.aj-kademe-cta b{color:var(--accent)}
.aj-sss{border-top:1px solid color-mix(in srgb,var(--ink) 14%,transparent)}
.aj-sss-madde{border-bottom:1px solid color-mix(in srgb,var(--ink) 14%,transparent)}
.aj-sss-madde summary{display:flex;align-items:center;justify-content:space-between;gap:16px;padding:18px 2px;font-size:clamp(13.5px,1.35vw,16.5px);font-weight:600;letter-spacing:-.01em;cursor:pointer;list-style:none}
.aj-sss-madde summary::-webkit-details-marker{display:none}
.aj-sss-madde summary::after{content:"+";flex:0 0 auto;font-size:20px;line-height:1;color:var(--accent);transition:transform .3s var(--aj-ease)}
.aj-sss-madde[open] summary::after{transform:rotate(45deg)}
.aj-sss-madde p{margin:0;padding:0 2px 20px;max-width:66ch;font-size:13px;line-height:1.8;color:color-mix(in srgb,var(--ink) 62%,transparent)}
.aj-guven-cta{display:flex;flex-wrap:wrap;align-items:center;gap:16px;margin-top:clamp(28px,3.4vw,42px)}
.aj-guven-cta .buton{padding:14px 22px}
/* ── Sektörel tasarım rejimleri ─────────────────────────────────────── */
/* Acil: agresif neon, hızlı aksiyon, sıkı ritim. */
body[data-stil="neon-acil"]{--aj-ease:cubic-bezier(.16,.94,.22,1)}
body[data-stil="neon-acil"] .aj-manset{font-weight:700;letter-spacing:-.05em}
body[data-stil="neon-acil"] .aj-kicker{color:var(--accent);letter-spacing:.3em}
body[data-stil="neon-acil"] .aj-buton{border-radius:10px;font-size:11.5px;text-transform:uppercase;letter-spacing:.12em;animation:ajNefes 2.6s ease-in-out infinite}
body[data-stil="neon-acil"] .aj-hizmet,body[data-stil="neon-acil"] .aj-kademe{border-radius:var(--r-2)}
body[data-stil="neon-acil"] .aj-hero-perde{background:linear-gradient(90deg,color-mix(in srgb,var(--bg) 96%,transparent) 0%,color-mix(in srgb,var(--bg) 62%,transparent) 52%,transparent 84%),linear-gradient(0deg,var(--bg) 4%,transparent 52%)}
body[data-stil="neon-acil"] .aj-rozet{border-color:color-mix(in srgb,var(--accent) 48%,transparent);background:color-mix(in srgb,var(--accent) 12%,transparent)}
body[data-stil="neon-acil"] .aj-no{color:var(--accent-2)}
@keyframes ajNefes{0%,100%{box-shadow:0 0 0 1px color-mix(in srgb,var(--accent) 55%,transparent),0 16px 40px -26px var(--accent)}50%{box-shadow:0 0 0 1px color-mix(in srgb,var(--accent) 92%,transparent),0 26px 66px -18px var(--accent)}}
/* Lüks: derin karanlık, zarif serif, sakin sinematik ritim. */
body[data-stil="luxe-sinema"]{--aj-ease:cubic-bezier(.22,.61,.24,1)}
body[data-stil="luxe-sinema"] .aj-manset{font-weight:400;line-height:.92;letter-spacing:-.05em}
body[data-stil="luxe-sinema"] .aj-kelime i{transition-duration:1.6s}
body[data-stil="luxe-sinema"] .aj-kicker,body[data-stil="luxe-sinema"] .aj-eyebrow{font-size:9.5px;letter-spacing:.34em}
body[data-stil="luxe-sinema"] .aj-bolum{padding:clamp(124px,13vw,196px) 0}
body[data-stil="luxe-sinema"] .aj-hizmet h3,body[data-stil="luxe-sinema"] .aj-kademe h3{font-size:clamp(24px,2.6vw,36px);font-weight:400}
body[data-stil="luxe-sinema"] .aj-cam{border-color:color-mix(in srgb,var(--accent) 16%,transparent);background:linear-gradient(150deg,color-mix(in srgb,var(--bg) 82%,transparent),color-mix(in srgb,var(--bg) 52%,transparent))}
body[data-stil="luxe-sinema"] .aj-buton{box-shadow:0 22px 60px -30px var(--accent)}
/* Temiz cam: dengeli hiyerarşi, ferah boşluk, kurumsal güven. */
body[data-stil="temiz-cam"] .aj-manset{font-weight:600;line-height:.92;letter-spacing:-.05em}
body[data-stil="temiz-cam"] .aj-cam{background:color-mix(in srgb,var(--bg) 68%,transparent);border-color:color-mix(in srgb,var(--ink) 16%,transparent);box-shadow:0 18px 50px -34px rgba(0,0,0,.55)}
body[data-stil="temiz-cam"] .aj-hizmet,body[data-stil="temiz-cam"] .aj-kademe{border-radius:var(--r-3)}
body[data-stil="temiz-cam"] .aj-bolum-ust p,body[data-stil="temiz-cam"] .aj-hero-alt{max-width:56ch}
body[data-stil="temiz-cam"] .aj-eyebrow{color:var(--accent)}
/* Atletik teknik: keskin ızgara, net çizgiler, mono etiketler. */
body[data-stil="atletik-teknik"] .aj-manset{font-size:clamp(44px,7.6vw,124px);font-weight:700;letter-spacing:-.05em;text-transform:uppercase}
body[data-stil="atletik-teknik"] .aj-hizmet,body[data-stil="atletik-teknik"] .aj-kademe,body[data-stil="atletik-teknik"] .aj-cam{border-radius:var(--r-1)}
body[data-stil="atletik-teknik"] .aj-eyebrow,body[data-stil="atletik-teknik"] .aj-kicker{letter-spacing:.18em}
body[data-stil="atletik-teknik"] .aj-hizmet{border-left:2px solid color-mix(in srgb,var(--accent) 46%,transparent)}
body[data-stil="atletik-teknik"] .aj-hizmet-no{color:var(--accent)}
/* ── Rejim başına bento oranları: hiçbir sektör aynı ızgarayı kopyalamaz ── */
@media (min-width:1001px){
body[data-stil="neon-acil"] .aj-hizmet:nth-child(1){grid-column:span 8}
body[data-stil="neon-acil"] .aj-hizmet:nth-child(2){grid-column:span 4}
body[data-stil="neon-acil"] .aj-hizmet:nth-child(3){grid-column:span 5}
body[data-stil="neon-acil"] .aj-hizmet:nth-child(4){grid-column:span 7}
body[data-stil="neon-acil"] .aj-is:nth-child(1){grid-column:span 8}
body[data-stil="neon-acil"] .aj-is:nth-child(2),body[data-stil="neon-acil"] .aj-is:nth-child(3){grid-column:span 4}
body[data-stil="neon-acil"] .aj-kademe:nth-child(1){grid-column:span 6}
body[data-stil="neon-acil"] .aj-kademe:nth-child(2){grid-column:span 4}
body[data-stil="neon-acil"] .aj-kademe:nth-child(3){grid-column:span 2}
body[data-stil="luxe-sinema"] .aj-hizmet:nth-child(1){grid-column:span 7;min-height:clamp(360px,42vw,540px)}
body[data-stil="luxe-sinema"] .aj-hizmet:nth-child(2){grid-column:span 5}
body[data-stil="luxe-sinema"] .aj-hizmet:nth-child(3){grid-column:span 4}
body[data-stil="luxe-sinema"] .aj-hizmet:nth-child(4){grid-column:span 8}
body[data-stil="luxe-sinema"] .aj-is:nth-child(1){grid-column:span 5}
body[data-stil="luxe-sinema"] .aj-is:nth-child(2),body[data-stil="luxe-sinema"] .aj-is:nth-child(3){grid-column:span 7}
body[data-stil="luxe-sinema"] .aj-kademe:nth-child(1){grid-column:span 4}
body[data-stil="luxe-sinema"] .aj-kademe:nth-child(2){grid-column:span 5}
body[data-stil="luxe-sinema"] .aj-kademe:nth-child(3){grid-column:span 3}
body[data-stil="temiz-cam"] .aj-hizmet:nth-child(1){grid-column:span 9}
body[data-stil="temiz-cam"] .aj-hizmet:nth-child(2){grid-column:span 3}
body[data-stil="temiz-cam"] .aj-hizmet:nth-child(3){grid-column:span 6}
body[data-stil="temiz-cam"] .aj-hizmet:nth-child(4){grid-column:span 6}
body[data-stil="temiz-cam"] .aj-is:nth-child(1){grid-column:span 5}
body[data-stil="temiz-cam"] .aj-is:nth-child(2),body[data-stil="temiz-cam"] .aj-is:nth-child(3){grid-column:span 7}
body[data-stil="temiz-cam"] .aj-kademe:nth-child(1){grid-column:span 5}
body[data-stil="temiz-cam"] .aj-kademe:nth-child(2){grid-column:span 3}
body[data-stil="temiz-cam"] .aj-kademe:nth-child(3){grid-column:span 4}
body[data-stil="atletik-teknik"] .aj-hizmet:nth-child(1){grid-column:span 10}
body[data-stil="atletik-teknik"] .aj-hizmet:nth-child(2){grid-column:span 2}
body[data-stil="atletik-teknik"] .aj-hizmet:nth-child(3){grid-column:span 3}
body[data-stil="atletik-teknik"] .aj-hizmet:nth-child(4){grid-column:span 9}
body[data-stil="atletik-teknik"] .aj-is:nth-child(1){grid-column:span 9}
body[data-stil="atletik-teknik"] .aj-is:nth-child(2),body[data-stil="atletik-teknik"] .aj-is:nth-child(3){grid-column:span 3}
body[data-stil="atletik-teknik"] .aj-kademe:nth-child(1){grid-column:span 7}
body[data-stil="atletik-teknik"] .aj-kademe:nth-child(2){grid-column:span 3}
body[data-stil="atletik-teknik"] .aj-kademe:nth-child(3){grid-column:span 2}
}
/* ── Kinetik derinlik, kırık ızgara ve yüzen cam paneller ───────────── */
.aj-kinetik{position:fixed;inset:-8vh 0;z-index:0;pointer-events:none;overflow:hidden;opacity:.55;transform:translate3d(0,var(--aj-par,0px),0)}
.aj-kinetik-ic{display:flex;align-items:center;gap:7vw;height:100%;width:max-content;animation:ajKinetik 82s linear infinite}
.aj-kinetik-ic span{font-family:var(--f-baslik);font-size:clamp(150px,28vw,430px);line-height:.78;letter-spacing:-.05em;text-transform:uppercase;white-space:nowrap;color:transparent;-webkit-text-stroke:1px color-mix(in srgb,var(--accent) 20%,transparent)}
@keyframes ajKinetik{from{transform:translate3d(0,0,0)}to{transform:translate3d(-50%,0,0)}}
.aj-diag{position:absolute;left:-10vw;top:32%;z-index:1;display:flex;gap:6vw;width:150%;transform:rotate(-8deg);pointer-events:none;opacity:.55}
.aj-diag span{font-family:var(--f-baslik);font-size:clamp(96px,15.5vw,260px);line-height:.78;letter-spacing:-.05em;text-transform:uppercase;white-space:nowrap;color:transparent;-webkit-text-stroke:1px color-mix(in srgb,var(--accent) 32%,transparent)}
.aj-hero-kopya{position:relative}
.aj-yuzen{position:absolute;left:2px;bottom:-10px;z-index:3;display:flex;flex-direction:column;gap:2px;padding:10px 15px;border-radius:14px;background:color-mix(in srgb,var(--bg) 48%,transparent);-webkit-backdrop-filter:blur(26px) saturate(1.4);backdrop-filter:blur(26px) saturate(1.4);border:1px solid rgba(255,255,255,.14);box-shadow:0 34px 90px -44px var(--accent);transform:translateY(46%) rotate(-2.6deg)}
.aj-yuzen span{font-size:9px;letter-spacing:.15em;text-transform:uppercase;color:color-mix(in srgb,var(--ink) 52%,transparent)}
.aj-yuzen b{font-family:var(--f-baslik);font-size:15px;font-weight:500;letter-spacing:-.01em}
body[data-glass="0"] .aj-yuzen{-webkit-backdrop-filter:blur(26px) saturate(1.4);backdrop-filter:blur(26px) saturate(1.4);background:color-mix(in srgb,var(--bg) 62%,transparent)}
.aj-manifesto-gorsel{margin-top:42px;margin-left:-7%;rotate:-1.5deg}
.aj-manifesto-metin{position:relative;z-index:2}
.aj-hizmet:nth-child(2){margin-top:-16px}
.aj-hizmet:nth-child(3){margin-left:-16px}
.aj-hizmet:nth-child(4){margin-top:10px;margin-left:14px}
.aj-mikro-bar{margin-left:-14px;margin-right:14px}
.aj-is:nth-child(2){margin-top:-20px}
.aj-is:nth-child(3){margin-top:12px}
@media (max-width:1000px){
 .aj-diag{display:none}
 .aj-yuzen{position:static;transform:none;margin-top:18px;width:max-content}
 .aj-manifesto-gorsel{margin-left:0;margin-top:26px;rotate:0deg}
 .aj-hizmet:nth-child(n),.aj-is:nth-child(n){margin-left:0;margin-top:0}
 .aj-mikro-bar{margin-left:0;margin-right:0}
}
/* ── Mobil: kompakt akış ve dokunmatik yatay kaydırma ───────────────── */
@media (max-width:1000px){
 .aj-glow{display:none}
 .aj-sayaclar{grid-template-columns:repeat(2,minmax(0,1fr))}
 .aj-sayac:nth-child(n){grid-column:auto}
 .aj-guven-rozetler{flex-wrap:nowrap;overflow-x:auto;scrollbar-width:none;-webkit-overflow-scrolling:touch;padding-bottom:4px}
 .aj-guven-rozetler::-webkit-scrollbar{display:none}
 .aj-kademeler{display:flex;gap:10px;overflow-x:auto;scroll-snap-type:x mandatory;padding-bottom:10px;scrollbar-width:none;-webkit-overflow-scrolling:touch}
 .aj-kademeler::-webkit-scrollbar{display:none}
 .aj-kademe{flex:0 0 min(82%,300px);scroll-snap-align:start}
 .aj-is-grid{display:flex;gap:10px;overflow-x:auto;scroll-snap-type:x mandatory;padding-bottom:10px;scrollbar-width:none;-webkit-overflow-scrolling:touch}
 .aj-is-grid::-webkit-scrollbar{display:none}
 .aj-is,.aj-is:first-child{flex:0 0 78%;min-height:300px;scroll-snap-align:start}
 .aj-sss-madde summary{padding:16px 0;gap:12px}
 .aj-guven-cta{flex-direction:column;align-items:stretch}
 .aj-guven-cta .buton{justify-content:center}
 body.acil-bar .mobil-bar{transition:transform .35s var(--aj-ease),opacity .35s}
 body.acil-bar .mobil-bar.gizli{transform:translateY(140%);opacity:0}
}
@media (max-width:560px){
 .aj-kademe{flex-basis:88%}
 .aj-is,.aj-is:first-child{flex-basis:86%}
 .aj-sayac b{font-size:clamp(28px,9vw,40px)}
}
"####;

const AJANS_JS: &str = r####"<script>
(function () {
  var azalt = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  document.body.classList.add("aj-hazir");

  // 1) Bölüm açılımı: IntersectionObserver, JS yoksa noscript görünür bırakır.
  var parcalar = [].slice.call(document.querySelectorAll("[data-aj-reveal]"));
  if (!azalt && "IntersectionObserver" in window) {
    var gozcu = new IntersectionObserver(function (kayitlar) {
      kayitlar.forEach(function (kayit) {
        if (kayit.isIntersecting) {
          kayit.target.classList.add("gorunur");
          gozcu.unobserve(kayit.target);
        }
      });
    }, { threshold: 0.12, rootMargin: "0px 0px -8% 0px" });
    parcalar.forEach(function (parca) { gozcu.observe(parca); });
  } else {
    parcalar.forEach(function (parca) { parca.classList.add("gorunur"); });
  }

  // 2) Okuma ilerlemesi + üst çubuk.
  var ilerleme = document.getElementById("aj-ilerleme");
  var ust = document.getElementById("ust");
  var kaydir = function () {
    var toplam = document.documentElement.scrollHeight - window.innerHeight;
    var oran = toplam > 0 ? Math.min(1, window.scrollY / toplam) : 0;
    if (ilerleme) ilerleme.style.width = (oran * 100).toFixed(2) + "%";
    if (ust) ust.classList.toggle("kucuk", window.scrollY > 18);
  };
  kaydir();
  window.addEventListener("scroll", kaydir, { passive: true });

  // 3) Parallax: hero arka planı kaydırmayla hafifçe kayar.
  var paralar = [].slice.call(document.querySelectorAll("[data-aj-par]"));
  if (paralar.length && !azalt) {
    var bekliyor = false;
    window.addEventListener("scroll", function () {
      if (bekliyor) return;
      bekliyor = true;
      window.requestAnimationFrame(function () {
        paralar.forEach(function (katman) {
          var oran = parseFloat(katman.getAttribute("data-aj-par")) || 0;
          katman.style.setProperty("--aj-par", (window.scrollY * oran).toFixed(2) + "px");
        });
        bekliyor = false;
      });
    }, { passive: true });
  }

  // 4) Mobil menü.
  var dugme = document.getElementById("mobil-dugme");
  var menu = document.getElementById("mobil-menu");
  if (dugme && menu) {
    dugme.addEventListener("click", function () {
      var acik = menu.classList.toggle("acik");
      dugme.setAttribute("aria-expanded", acik ? "true" : "false");
    });
    [].forEach.call(menu.querySelectorAll("a"), function (a) {
      a.addEventListener("click", function () { menu.classList.remove("acik"); });
    });
  }

  // 5) Manyetik butonlar ve 3B kart eğimi.
  if (!azalt && window.matchMedia("(hover: hover) and (pointer: fine)").matches) {
    [].forEach.call(document.querySelectorAll("[data-aj-magnetic]"), function (dugme) {
      dugme.addEventListener("pointermove", function (e) {
        var kutu = dugme.getBoundingClientRect();
        var x = (e.clientX - kutu.left - kutu.width / 2) * 0.16;
        var y = (e.clientY - kutu.top - kutu.height / 2) * 0.16;
        dugme.style.transform = "translate(" + x.toFixed(1) + "px," + y.toFixed(1) + "px)";
      });
      dugme.addEventListener("pointerleave", function () { dugme.style.transform = ""; });
    });
    [].forEach.call(document.querySelectorAll("[data-aj-tilt]"), function (kart) {
      kart.addEventListener("pointermove", function (e) {
        var kutu = kart.getBoundingClientRect();
        var x = (e.clientX - kutu.left) / kutu.width - 0.5;
        var y = (e.clientY - kutu.top) / kutu.height - 0.5;
        kart.style.setProperty("--rx", (-y * 4).toFixed(2) + "deg");
        kart.style.setProperty("--ry", (x * 4).toFixed(2) + "deg");
        kart.style.setProperty("--aj-kx", ((x + 0.5) * 100).toFixed(1) + "%");
        kart.style.setProperty("--aj-ky", ((y + 0.5) * 100).toFixed(1) + "%");
      });
      kart.addEventListener("pointerleave", function () {
        kart.style.setProperty("--rx", "0deg");
        kart.style.setProperty("--ry", "0deg");
      });
    });
  }

  // 6) İmleci izleyen nokta.
  var imlec = document.getElementById("aj-imlec");
  if (imlec && !azalt && window.matchMedia("(pointer: fine)").matches) {
    window.addEventListener("pointermove", function (e) {
      imlec.style.left = e.clientX + "px";
      imlec.style.top = e.clientY + "px";
    }, { passive: true });
    [].forEach.call(document.querySelectorAll("a, button"), function (oge) {
      oge.addEventListener("pointerenter", function () { imlec.classList.add("buyuk"); });
      oge.addEventListener("pointerleave", function () { imlec.classList.remove("buyuk"); });
    });
  }

  // 7) Yerel saat.
  var saat = document.getElementById("aj-saat");
  if (saat) {
    var yaz = function () {
      saat.textContent = new Date().toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });
    };
    yaz();
    setInterval(yaz, 60000);
  }

  // 8) Form: WhatsApp taslağı.
  var form = document.getElementById("aj-form");
  if (form) {
    form.addEventListener("submit", function (e) {
      e.preventDefault();
      var not = document.getElementById("aj-form-not");
      var ad = form.elements.ad.value.trim();
      var tel = form.elements.tel.value.trim();
      var mesaj = form.elements.mesaj.value.trim();
      if (!ad || !tel) {
        if (not) not.textContent = "Ad ve telefon gerekli.";
        return;
      }
      var wa = form.getAttribute("data-wa");
      if (!wa) {
        if (not) not.textContent = "Bu taslakta telefon tanımlı değil.";
        return;
      }
      var metin = "Merhaba, " + ad + " (" + tel + "). " + (mesaj || "Demo hakkında bilgi almak istiyorum.");
      window.open("https://wa.me/" + wa + "?text=" + encodeURIComponent(metin), "_blank", "noopener");
      if (not) not.textContent = "WhatsApp açılıyor…";
    });
  }

  // 9) İmleç ışığı ve hero paralaksı: yalnız hassas imleçte, hafif.
  var isik = document.getElementById("aj-glow");
  var ince = window.matchMedia("(hover: hover) and (pointer: fine)").matches;
  if (!azalt && ince) {
    var imlecX = window.innerWidth / 2;
    var imlecY = window.innerHeight * 0.3;
    var isikBekliyor = false;
    var isikYaz = function () {
      isikBekliyor = false;
      if (isik) {
        isik.style.setProperty("--aj-mx", imlecX.toFixed(0) + "px");
        isik.style.setProperty("--aj-my", imlecY.toFixed(0) + "px");
      }
      var yatay = imlecX / window.innerWidth - 0.5;
      var dikey = imlecY / window.innerHeight - 0.5;
      [].forEach.call(document.querySelectorAll("[data-aj-pointer]"), function (katman) {
        katman.style.setProperty("--aj-px", (-yatay * 18).toFixed(1) + "px");
        katman.style.setProperty("--aj-py", (-dikey * 14).toFixed(1) + "px");
      });
    };
    if (isik) document.body.classList.add("aj-isik");
    window.addEventListener("pointermove", function (e) {
      imlecX = e.clientX;
      imlecY = e.clientY;
      if (!isikBekliyor) {
        isikBekliyor = true;
        window.requestAnimationFrame(isikYaz);
      }
    }, { passive: true });
  }

  // 10) Memnuniyet sayaçları: görünür olunca yumuşak sayım.
  var bicim = function (deger, ondalik) {
    return deger.toLocaleString("tr-TR", { minimumFractionDigits: ondalik, maximumFractionDigits: ondalik });
  };
  var sayaclar = [].slice.call(document.querySelectorAll("[data-aj-sayac]"));
  if (sayaclar.length) {
    var sayacBasla = function (oge) {
      var hedefSayi = parseFloat(oge.getAttribute("data-aj-sayac")) || 0;
      var ondalik = parseInt(oge.getAttribute("data-aj-ondalik") || "0", 10);
      if (azalt) {
        oge.textContent = bicim(hedefSayi, ondalik);
        return;
      }
      var baslangic = 0;
      var adim = function (an) {
        if (!baslangic) baslangic = an;
        var oran = Math.min(1, (an - baslangic) / 1500);
        oge.textContent = bicim(hedefSayi * (1 - Math.pow(1 - oran, 3)), ondalik);
        if (oran < 1) window.requestAnimationFrame(adim);
      };
      oge.textContent = bicim(0, ondalik);
      window.requestAnimationFrame(adim);
    };
    if (!azalt && "IntersectionObserver" in window) {
      var sayacGozcu = new IntersectionObserver(function (kayitlar) {
        kayitlar.forEach(function (kayit) {
          if (kayit.isIntersecting) {
            sayacBasla(kayit.target);
            sayacGozcu.unobserve(kayit.target);
          }
        });
      }, { threshold: 0.55 });
      sayaclar.forEach(function (oge) { sayacGozcu.observe(oge); });
    } else {
      sayaclar.forEach(sayacBasla);
    }
  }

  // 11) SSS akordeonu: tek seferde tek yanıt açık kalır.
  var sssMaddeleri = [].slice.call(document.querySelectorAll(".aj-sss-madde"));
  sssMaddeleri.forEach(function (madde) {
    madde.addEventListener("toggle", function () {
      if (!madde.open) return;
      sssMaddeleri.forEach(function (diger) {
        if (diger !== madde && diger.open) diger.open = false;
      });
    });
  });

  // 12) Mobil eylem barı: aşağı kaydırırken saklanır, yukarıda geri gelir.
  var mobilBar = document.querySelector(".mobil-bar");
  if (mobilBar) {
    var sonKaydirma = window.scrollY;
    window.addEventListener("scroll", function () {
      var simdiki = window.scrollY;
      mobilBar.classList.toggle("gizli", simdiki > sonKaydirma + 8 && simdiki > 240);
      sonKaydirma = simdiki;
    }, { passive: true });
  }
})();
</script>
"####;

// ---------------------------------------------------------------------------
// Sinematik profil
//
// Ajans profilinden tamamen ayrı bir sahne motoru: sayfa "sticky" sahneler
// hâlinde kayar, arkada küçük bir WebGL shader'ı ışık üretir, sahnede sektöre
// göre seçilen CSS 3D nesne döner. Harici kütüphane, CDN ve video yoktur;
// JavaScript kapalıysa sahneler normal bölüm akışına döner.
// ---------------------------------------------------------------------------

/// Doğrulanmış hex rengi döndürür; geçersiz değer yedeğe düşer.
fn renk_veya(deger: &str, yedek: &str) -> String {
    let deger = deger.trim();
    let govde = deger.strip_prefix('#').unwrap_or("");
    let gecerli = (govde.len() == 3 || govde.len() == 6)
        && govde.chars().all(|harf| harf.is_ascii_hexdigit());
    if gecerli {
        format!("#{}", govde.to_ascii_lowercase())
    } else {
        yedek.to_string()
    }
}

/// Panel değerini üç kademeye indirger; bilinmeyen değer "orta".
fn hareket_sinifi(deger: &str) -> &'static str {
    match deger.trim().to_ascii_lowercase().as_str() {
        "dusuk" | "düşük" => "dusuk",
        "yuksek" | "yüksek" => "yuksek",
        _ => "orta",
    }
}

/// Sektör grubuna göre 3D nesne; panelde elle seçim varsa o kazanır.
fn nesne_tipi(grup: &str, secim: &str) -> &'static str {
    match secim.trim().to_ascii_lowercase().as_str() {
        "halka" => return "halka",
        "kup" => return "kup",
        "dalga" => return "dalga",
        "geometri" => return "geometri",
        _ => {}
    }
    match grup {
        "acil" | "saglik" => "halka",
        "mutfak" | "konaklama" | "egitim" => "kup",
        "guzellik" | "servis" | "kurumsal" => "geometri",
        "spor" | "etkinlik" => "dalga",
        _ => "dalga",
    }
}

/// İşletme adı + adresten deterministik 6 haneli tohum (FNV-1a).
/// Aynı işletme her üretimde aynı kompozisyonu alır; komşu işletme farklı.
fn tohum(ad: &str, adres: &str) -> String {
    let mut ozet: u32 = 0x811c_9dc5;
    for bayt in ad.bytes().chain([0u8]).chain(adres.bytes()) {
        ozet ^= u32::from(bayt);
        ozet = ozet.wrapping_mul(0x0100_0193);
    }
    format!("{:06x}", ozet & 0x00ff_ffff)
}

/// Manşeti harf harf maskeden açılan satırlara çevirir. Ekran okuyucu için
/// her satırın düz metni ayrıca (görünmez) yazılır.
fn sn_satirlar(metin: &str) -> String {
    let mut cikti = String::new();
    for (sira, satir) in metin.lines().filter(|s| !s.trim().is_empty()).enumerate() {
        let satir = satir.trim();
        let mut harfler = String::new();
        for (sira_harf, harf) in satir.chars().enumerate() {
            let parca = if harf == ' ' {
                "&nbsp;".to_string()
            } else {
                kacis(&harf.to_string())
            };
            harfler.push_str(&format!(
                r##"<i class="sn-h" style="--i:{sira_harf}">{parca}</i>"##
            ));
        }
        cikti.push_str(&format!(
            r##"<span class="sn-satir" style="--g:{}"><span class="sn-harf-kap" aria-hidden="true">{harfler}</span><span class="sn-ekran">{}</span></span>"##,
            sira * 90,
            kacis(satir)
        ));
    }
    cikti
}

/// İşletmeye özel sinematik arka plan dünyası: bütün alanlar işletme adı ve
/// adresinden türeyen tohumdan gelir. Aynı sektörde iki işletme asla aynı
/// akışı, aynı hızı ve aynı partikül yoğunluğunu almaz.
#[derive(Clone, Debug)]
pub struct Dunya {
    pub tohum: u64,
    pub aci: u32,
    pub aci2: u32,
    /// Akışkan dalganın hız çarpanı (0,55–1,45).
    pub hiz: f64,
    /// Dalga frekansı çarpanı (0,72–1,58).
    pub dalga: f64,
    /// Partikül yoğunluğu (0,16–0,78).
    pub parca: f64,
    /// Vinyet koyuluğu (0,34–0,66).
    pub vinyet: f64,
    /// Birinci aurora rengi (vurgudan türetilir).
    pub akis1: String,
    /// İkinci aurora rengi (zemine doğru karıştırılır).
    pub akis2: String,
}

/// Tohumdan deterministik dünya üretir; aynı işletme her üretimde aynı
/// görsel kimliği, komşu işletme farklı kimliği alır.
pub fn dunya_uret(tohum: &str, vurgu: &str, zemin: &str) -> Dunya {
    let sayi = u64::from_str_radix(tohum.trim(), 16).unwrap_or(0x5eed_1234);
    // Tohum yalnız 24 bit: bitleri karıştırıp üç bağımsız akışa ayırırız.
    let a = tohum_karistir(sayi);
    let b = tohum_karistir(a);
    let c = tohum_karistir(b);
    let aci = (a % 360) as u32;
    let aci2 = ((a >> 9) % 360) as u32;
    let hiz = 0.55 + (b % 100) as f64 / 100.0 * 0.9;
    let dalga = 0.72 + ((b >> 8) % 100) as f64 / 100.0 * 0.86;
    let parca = 0.16 + (c % 100) as f64 / 100.0 * 0.62;
    let vinyet = 0.34 + ((c >> 8) % 100) as f64 / 100.0 * 0.32;
    let akis1 = renk_karistir(vurgu, &renk_dondur(vurgu, f64::from(aci) * 0.6), 0.55);
    let akis2 = renk_karistir(
        zemin,
        &renk_dondur(vurgu, 180.0 + f64::from(aci2) * 0.5),
        0.62,
    );
    Dunya {
        tohum: sayi,
        aci,
        aci2,
        hiz,
        dalga,
        parca,
        vinyet,
        akis1,
        akis2,
    }
}

/// 64 bit karıştırıcı (SplitMix64): küçük tohumlarda bit dağılımı düzgün olur.
fn tohum_karistir(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Ken Burns (zoom + pan) değişkenleri: aynı kare her işletmede farklı
/// yönde, farklı hızda ve farklı odak noktasından akar.
pub fn ken_burns_stil(tohum: u64, sira: usize) -> String {
    let a = tohum_karistir(tohum ^ (sira as u64).wrapping_mul(0x9e37_79b9));
    let sure = 13 + (a % 9);
    let gecikme = (a >> 8) % 7;
    let olcek = 1.10 + ((a >> 12) % 12) as f64 * 0.01;
    let odak_x = 44 + (a >> 20) % 13;
    let odak_y = 42 + (a >> 27) % 17;
    let kayma_x = ((a >> 33) % 9) as i64 - 4;
    let kayma_y = ((a >> 41) % 9) as i64 - 4;
    format!(
        "--kb-s:{sure}s;--kb-d:-{gecikme}s;--kb-z:{olcek:.2};--kb-x:{kayma_x}%;--kb-y:{kayma_y}%;--kb-ox:{odak_x}%;--kb-oy:{odak_y}%"
    )
}

/// Kartlarda kullanılacak fotoğraflar: önce galeri, yoksa hero karesi.
fn kart_fotolari(g: &Girdi) -> Vec<(String, String)> {
    let mut liste: Vec<(String, String)> = g.galeri.iter().take(4).cloned().collect();
    if liste.is_empty() {
        if let Some(foto) = g.foto.as_ref() {
            liste.push(foto.clone());
        }
    }
    liste
}

/// Kartın hangi fotoğraf kuralını kullanacağı.
fn kart_foto_sinifi(g: &Girdi, sira: usize) -> String {
    let adet = kart_fotolari(g).len();
    if adet == 0 {
        String::new()
    } else {
        format!("sn-kf{}", sira % adet)
    }
}

/// Kart fotoğraflarının CSS kuralları: aynı kare tek kez gömülür.
fn kart_foto_kurallari(g: &Girdi) -> String {
    let mut kurallar = String::new();
    for (sira, (mime, veri)) in kart_fotolari(g).iter().enumerate() {
        kurallar.push_str(&format!(
            ".sn-kf{sira}{{background-image:url(data:{mime};base64,{veri})}}"
        ));
    }
    kurallar
}

/// Hero videosunun işletmeye özel açılışı: başlangıç saniyesi ve kırpma
/// odağı tohumdan gelir, aynı havuz videosu iki işletmede aynı görünmez.
fn video_stil(tohum: u64) -> String {
    let a = tohum_karistir(tohum ^ 0x0051_ed0f);
    let bas = (a % 480) as f64 / 100.0;
    let x = 40 + (a >> 9) % 21;
    let y = 40 + (a >> 19) % 21;
    format!("--sn-v-bas:{bas:.2};--sn-v-konum:{x}% {y}%")
}

/// `#rrggbb` (veya `#rgb`) rengi 0–1 aralığına çevirir.
fn renk_bilesenleri(hex: &str) -> Option<[f64; 3]> {
    let temiz = hex.trim().trim_start_matches('#');
    let genis = if temiz.len() == 3 {
        temiz.chars().flat_map(|k| [k, k]).collect::<String>()
    } else if temiz.len() == 6 {
        temiz.to_string()
    } else {
        return None;
    };
    let parca = |bas: usize| u8::from_str_radix(&genis[bas..bas + 2], 16).ok();
    Some([
        f64::from(parca(0)?) / 255.0,
        f64::from(parca(2)?) / 255.0,
        f64::from(parca(4)?) / 255.0,
    ])
}

fn renk_yaz(rgb: [f64; 3]) -> String {
    let kanal = |deger: f64| (deger.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        kanal(rgb[0]),
        kanal(rgb[1]),
        kanal(rgb[2])
    )
}

/// İki rengi orana göre karıştırır (`oran` ikinci rengin payı).
fn renk_karistir(a: &str, b: &str, oran: f64) -> String {
    let (Some(a), Some(b)) = (renk_bilesenleri(a), renk_bilesenleri(b)) else {
        return a.to_string();
    };
    let oran = oran.clamp(0.0, 1.0);
    renk_yaz([
        a[0] * (1.0 - oran) + b[0] * oran,
        a[1] * (1.0 - oran) + b[1] * oran,
        a[2] * (1.0 - oran) + b[2] * oran,
    ])
}

/// Rengin tonunu derece kadar döndürür (HSL üzerinden, doygunluk korunur).
fn renk_dondur(hex: &str, derece: f64) -> String {
    let Some([r, g, b]) = renk_bilesenleri(hex) else {
        return hex.to_string();
    };
    let mak = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (mak + min) / 2.0;
    let d = mak - min;
    if d < f64::EPSILON {
        return hex.to_string();
    }
    let s = if l > 0.5 {
        d / (2.0 - mak - min)
    } else {
        d / (mak + min)
    };
    let mut h = if (mak - r).abs() < f64::EPSILON {
        ((g - b) / d) % 6.0
    } else if (mak - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h = (h * 60.0 + derece).rem_euclid(360.0) / 360.0;
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let kanal = |t: f64| {
        let t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    renk_yaz([kanal(h + 1.0 / 3.0), kanal(h), kanal(h - 1.0 / 3.0)])
}

/// Sahnede duran 3D nesne. Hepsi saf CSS/SVG/Canvas; görsel dosyası yok.
fn nesne_html(tip: &str, ad: &str) -> String {
    match tip {
        "halka" => {
            let mut parcalar = String::new();
            for sira in 0..8 {
                parcalar.push_str(&format!(r##"<i style="--i:{sira}"></i>"##));
            }
            format!(
                r##"<div class="sn-nesne sn-halka" aria-hidden="true">{parcalar}<b>{}</b></div>"##,
                kacis(ad)
            )
        }
        "kup" => {
            let yuzler = ["on", "arka", "sag", "sol", "ust", "alt"];
            let mut parcalar = String::new();
            for yuz in yuzler {
                parcalar.push_str(&format!(r##"<i class="{yuz}"></i>"##));
            }
            format!(r##"<div class="sn-nesne sn-kup" aria-hidden="true">{parcalar}</div>"##)
        }
        "geometri" => r##"<div class="sn-nesne sn-geometri" aria-hidden="true"><svg viewBox="0 0 240 240" fill="none" stroke="currentColor" stroke-width="1"><polygon points="120,18 214,74 214,166 120,222 26,166 26,74"/><polygon points="120,54 182,92 182,148 120,186 58,148 58,92"/><path d="M120 18v204M26 74l188 92M214 74L26 166"/><circle cx="120" cy="120" r="86" stroke-dasharray="4 8"/></svg></div>"##
            .to_string(),
        _ => r##"<div class="sn-nesne sn-dalga-kutu" aria-hidden="true"><canvas class="sn-dalga" width="600" height="380"></canvas></div>"##.to_string(),
    }
}

/// Sinematik sayfa: 3–6 sticky sahne, shader zemin, dönüşüm katmanı.
fn sinematik_sayfa(g: &Girdi, ic: &Icerik) -> String {
    let ad = kacis(&g.ad);
    let sektor = kacis(&g.sektor);
    let bolge = kacis(&ilce_metni(&g.adres));
    let adres = kacis(&g.adres);
    let brif = g.brif.as_ref();
    let tasarim = g.tasarim.as_ref();
    let grup = tasarim
        .map(|tasarim| tasarim.grup.as_str())
        .unwrap_or(if acil_sektor(&g.sektor) { "acil" } else { "" });
    let stil = brif
        .map(|b| b.stil.as_str())
        .filter(|stil| !stil.is_empty())
        .or_else(|| {
            tasarim.map(|t| crate::tasarim_brifi::stil_adi(t.grup.as_str(), t.aciliyet >= 2))
        })
        .unwrap_or("luxe-sinema");
    let vurgu = renk_veya(
        brif.map(|b| b.palet.vurgu.as_str())
            .filter(|renk| !renk.is_empty())
            .unwrap_or(g.vurgu.as_str()),
        "#c9a227",
    );
    let zemin = renk_veya(
        brif.map(|b| b.palet.zemin.as_str())
            .filter(|renk| !renk.is_empty())
            .unwrap_or(g.zemin.as_str()),
        "#08080a",
    );
    let yazi = renk_veya(
        brif.map(|b| b.palet.yazi.as_str())
            .filter(|renk| !renk.is_empty())
            .unwrap_or(g.yazi.as_str()),
        "#f5f3ef",
    );
    let baslik_yigin = brif
        .map(|b| b.tipografi.baslik.as_str())
        .filter(|font| !font.is_empty())
        .or_else(|| {
            tasarim
                .map(|t| t.baslik.as_str())
                .filter(|font| !font.is_empty())
        })
        .unwrap_or("'Instrument Serif','Space Grotesk',Georgia,serif");
    let govde_yigin = brif
        .map(|b| b.tipografi.govde.as_str())
        .filter(|font| !font.is_empty())
        .unwrap_or("'Geist',system-ui,-apple-system,'Segoe UI',sans-serif");

    let nesne = nesne_tipi(grup, &g.sinematik.nesne);
    let sahne_sayisi = g.sinematik.sahne.clamp(3, 6);
    let hareket = hareket_sinifi(&g.sinematik.hareket);
    let tohum = tohum(&g.ad, &g.adres);

    let telefon = g
        .telefon
        .as_deref()
        .map(str::trim)
        .filter(|tel| !tel.is_empty());
    let tel_hedef = telefon.map(|tel| format!("tel:+{}", tel.trim_start_matches('+')));
    let tel_goster = if g.telefon_goster.trim().is_empty() {
        "Telefonla bilgi".to_string()
    } else {
        kacis(&g.telefon_goster)
    };
    let wa_no: String = telefon
        .or(g.satici_tel.as_deref())
        .map(|tel| tel.chars().filter(char::is_ascii_digit).collect())
        .unwrap_or_default();
    let cta = if !g.cta.trim().is_empty() {
        g.cta.trim().to_string()
    } else if let Some(b) = brif.filter(|b| !b.cta.trim().is_empty()) {
        b.cta.trim().to_string()
    } else {
        "Randevu al".to_string()
    };
    let cta = kacis(&cta);
    let slogan = if !g.slogan.trim().is_empty() {
        kacis(&g.slogan)
    } else {
        kacis(ic.alt)
    };

    // Manşet: panel metni > brif başlığı > sektör + bölge.
    let manset = {
        let m1 = g.metin.manset1.trim();
        let m2 = g.metin.manset2.trim();
        if !m1.is_empty() || !m2.is_empty() {
            format!("{m1}\n{m2}")
        } else if let Some(baslik) = brif.and_then(|b| b.bolum(0)).map(|b| b.baslik.clone()) {
            baslik
        } else {
            format!("{sektor}\n{bolge}")
        }
    };
    let hakkinda_baslik = if !g.metin.hakkinda_baslik.trim().is_empty() {
        g.metin.hakkinda_baslik.trim().to_string()
    } else {
        brif.and_then(|b| b.bolum(1))
            .map(|b| b.baslik.clone())
            .unwrap_or_else(|| "Bir mekân değil,\nbir ritüel.".to_string())
    };
    let hakkinda_metin = if !g.metin.hakkinda1.trim().is_empty() {
        g.metin.hakkinda1.trim().to_string()
    } else {
        brif.and_then(|b| b.bolum(1))
            .map(|b| b.metin.clone())
            .filter(|metin| !metin.is_empty())
            .unwrap_or_else(|| slogan.clone())
    };
    let bolum_baslik = if !g.metin.bolum_baslik.trim().is_empty() {
        g.metin.bolum_baslik.trim().to_string()
    } else {
        brif.and_then(|b| b.bolum(2))
            .map(|b| b.baslik.clone())
            .unwrap_or_else(|| format!("{} · hizmetler", sektor))
    };
    let bolum_metin = if !g.metin.bolum_alt.trim().is_empty() {
        g.metin.bolum_alt.trim().to_string()
    } else {
        brif.and_then(|b| b.bolum(2))
            .map(|b| b.metin.clone())
            .unwrap_or_default()
    };

    // Hizmet satırları: panel > brif > sektör içeriği.
    let mut hizmetler: Vec<(String, String)> = Vec::new();
    if let Some(bolum) = brif.and_then(|b| b.bolum(2)) {
        for kalem in bolum.kalemler.iter().take(4) {
            if !kalem.ad.trim().is_empty() {
                hizmetler.push((kalem.ad.clone(), kalem.aciklama.clone()));
            }
        }
    }
    if hizmetler.is_empty() {
        for (sira, (kalem, aciklama)) in ic.kalemler.iter().take(4).enumerate() {
            let panel_ad = g.metin.satir(&g.metin.kalemler, sira, 0).trim();
            let panel_ack = g.metin.satir(&g.metin.kalemler, sira, 1).trim();
            hizmetler.push((
                if panel_ad.is_empty() {
                    (*kalem).to_string()
                } else {
                    panel_ad.to_string()
                },
                if panel_ack.is_empty() {
                    (*aciklama).to_string()
                } else {
                    panel_ack.to_string()
                },
            ));
        }
    }

    let bolge_satiri = format!("{} · {}", sektor, bolge);
    let yol_baglantisi = g
        .konum
        .map(|(enlem, boylam)| format!("https://www.google.com/maps?q={enlem:.5},{boylam:.5}"))
        .or_else(|| {
            (!g.adres.trim().is_empty()).then(|| {
                format!(
                    "https://www.google.com/maps/search/?api=1&query={}",
                    url_kacis(&g.adres)
                )
            })
        });

    // --- Sahneler -------------------------------------------------------
    let mut sahne_listesi: Vec<(&str, String)> = Vec::new();

    // 0 · hero
    let rozet = if g.program.is_some() {
        r##"<span class="sn-durum" id="sn-durum" hidden></span>"##
    } else {
        ""
    };
    // Hero medyası tam ekran bleed katmanına gider; nesne yalnız medya yokken
    // metnin altında kalır.
    let (hero_medya, hero_nesne) = match (&g.video, &g.foto) {
        (Some(video), _) => (
            format!(
                r##"<figure class="sn-hero-foto sn-hero-video" style="--sn-poster:url('{}');{stil}"><video autoplay loop muted playsinline class="sn-video bg-video" id="sn-video" preload="auto" disablepictureinpicture poster="{}" aria-label="{ad} — {sektor}"><source src="{}" type="video/mp4"></video><span class="sn-karart" aria-hidden="true"></span></figure>"##,
                kacis(&video.poster),
                kacis(&video.poster),
                kacis(&video.dosya),
                stil = video_stil(g.medya_tohum)
            ),
            String::new(),
        ),
        (None, Some((mime, veri))) => (
            format!(
                r##"<figure class="sn-hero-foto sn-ken" style="{kb}"><img src="data:{mime};base64,{veri}" alt="{ad} — {sektor}" loading="eager" decoding="async"></figure>"##,
                kb = ken_burns_stil(g.medya_tohum, 0)
            ),
            String::new(),
        ),
        (None, None) => (
            String::new(),
            format!(
                r##"<div class="sn-hero-sahne">{}</div>"##,
                nesne_html(nesne, &g.ad)
            ),
        ),
    };
    // Medya artık hero kutusunda değil: tüm sayfanın arkasında sabit duran
    // tek bir katman. Kaydırma boyunca klip arkada akar, içerik üstünden geçer.
    let zemin_katmani = if hero_medya.is_empty() {
        String::new()
    } else {
        format!(
            r##"<div class="sn-zemin-medya" aria-hidden="true">{hero_medya}<span class="sn-zemin-perde" aria-hidden="true"></span></div>"##
        )
    };
    let zemin_modu = if hero_medya.is_empty() {
        "duz"
    } else {
        "medya"
    };
    let hero_butonlar = {
        let mut cikti = String::new();
        if let Some(hedef) = &tel_hedef {
            cikti.push_str(&format!(
                r##"<a class="sn-buton dolu" href="{hedef}">{TELEFON_IKON}<span>{cta}</span></a>"##
            ));
        }
        if let Some(yol) = &yol_baglantisi {
            cikti.push_str(&format!(
                r##"<a class="sn-buton" href="{yol}" target="_blank" rel="noopener">{KONUM_IKON}<span>Yol tarifi</span></a>"##
            ));
        }
        cikti
    };
    sahne_listesi.push((
        "hero",
        format!(
            r##"<section class="sn-sahne sn-hero" data-sahne="0" aria-labelledby="sn-b0">
<div class="sn-sahne-ic sn-hero-grid hero-overlay">
<div class="sn-hero-metin">
<p class="sn-etiket">{rozet}{bolge_satiri}</p>
<h1 class="sn-h1" id="sn-b0">{manset_satirlari}</h1>
<p class="sn-lede">{slogan}</p>
<div class="sn-butonlar">{hero_butonlar}</div>
</div>
{hero_nesne}
</div>
<a class="sn-asagi" href="#sn-b1" aria-label="Aşağı kaydır"><i></i></a>
</section>"##,
            manset_satirlari = sn_satirlar(&manset),
        ),
    ));

    // 1 · manifesto
    sahne_listesi.push((
        "manifesto",
        format!(
            r##"<section class="sn-sahne" data-sahne="1" aria-labelledby="sn-b1">
<div class="sn-sahne-ic sn-iki">
<p class="sn-sayi" aria-hidden="true">01</p>
<div>
<p class="sn-etiket">Hakkında</p>
<h2 class="sn-h2" id="sn-b1">{hakkinda_satirlar}</h2>
<p class="sn-metin">{hakkinda_metin}</p>
</div>
</div>
</section>"##,
            hakkinda_satirlar = sn_satirlar(&hakkinda_baslik),
            hakkinda_metin = kacis(&hakkinda_metin),
        ),
    ));

    // 2 · hizmetler
    let mut hizmet_kartlari = String::new();
    for (sira, (kalem, aciklama)) in hizmetler.iter().enumerate() {
        // Kart da fotoğrafla açılır: aynı kareler Ken Burns ile akar.
        // Fotoğraf kuralları tek bir <style> bloğunda bir kez yazılır; her
        // kart aynı base64'ü tekrar etmez, sayfa bütçesi korunur.
        let kart_foto = if g.galeri.is_empty() && g.foto.is_none() {
            String::new()
        } else {
            format!(
                r##"<span class="sn-kart-foto {sinif}" style="{kb}" aria-hidden="true"></span>"##,
                sinif = kart_foto_sinifi(g, sira),
                kb = ken_burns_stil(g.medya_tohum, sira + 1)
            )
        };
        hizmet_kartlari.push_str(&format!(
            r##"<li class="sn-kart" data-egil style="--g:{g};" data-foto="1"><span class="sn-kart-no">0{sira}</span>{foto}<h3>{kalem}</h3><p>{aciklama}</p></li>"##,
            g = sira * 80,
            sira = sira + 1,
            kalem = kacis(kalem),
            aciklama = kacis(aciklama),
            foto = kart_foto
        ));
    }
    sahne_listesi.push((
        "hizmet",
        format!(
            r##"<section class="sn-sahne" data-sahne="2" aria-labelledby="sn-b2">
<div class="sn-sahne-ic">
<p class="sn-etiket">Hizmetler</p>
<h2 class="sn-h2" id="sn-b2">{baslik}</h2>
{metin}
<ul class="sn-kartlar">{hizmet_kartlari}</ul>
</div>
</section>"##,
            baslik = sn_satirlar(&bolum_baslik),
            metin = if bolum_metin.trim().is_empty() {
                String::new()
            } else {
                format!(r##"<p class="sn-metin">{}</p>"##, kacis(&bolum_metin))
            },
        ),
    ));

    // 3 · galeri (kapalıysa veya kare yoksa hiç eklenmez)
    if g.bolumler.galeri && !g.galeri.is_empty() {
        let mut kareler = String::new();
        for (sira, (mime, veri)) in g.galeri.iter().take(2).enumerate() {
            kareler.push_str(&format!(
                r##"<figure class="sn-kare" style="--g:{};{kb}"><img src="data:{mime};base64,{veri}" alt="{ad} — fotoğraf {}" loading="lazy" decoding="async"></figure>"##,
                sira * 90,
                sira + 1,
                kb = ken_burns_stil(g.medya_tohum, sira + 2)
            ));
        }
        let kaynak = g
            .galeri_not
            .as_deref()
            .or(g.galeri_atif.as_deref())
            .unwrap_or("İşletme fotoğrafları");
        sahne_listesi.push((
            "galeri",
            format!(
                r##"<section class="sn-sahne" data-sahne="3" aria-labelledby="sn-b3">
<div class="sn-sahne-ic">
<p class="sn-etiket"><span>Galeri</span><span>{kaynak}</span></p>
<h2 class="sn-h2" id="sn-b3">{ad}</h2>
<div class="sn-galeri">{kareler}</div>
</div>
</section>"##,
                kaynak = kacis(kaynak)
            ),
        ));
    }

    // 4 · saatler + adres (veri yoksa çizilmez)
    let saat_verisi = !g.saat_satirlari.is_empty() || !g.saat_metni.trim().is_empty();
    if (g.bolumler.saatler && saat_verisi) || yol_baglantisi.is_some() {
        let mut satirlar = String::new();
        for (gun, saat) in g.saat_satirlari.iter() {
            satirlar.push_str(&format!(
                r##"<div class="sn-saat-satir"><span>{}</span><b>{}</b></div>"##,
                kacis(gun),
                kacis(saat)
            ));
        }
        if satirlar.is_empty() && !g.saat_metni.trim().is_empty() {
            satirlar.push_str(&format!(
                r##"<div class="sn-saat-satir"><span>Çalışma saatleri</span><b>{}</b></div>"##,
                kacis(&g.saat_metni)
            ));
        }
        let saat_blogu = if g.bolumler.saatler && !satirlar.is_empty() {
            format!(r##"<div class="sn-saatler">{satirlar}</div>"##)
        } else {
            String::new()
        };
        let adres_blogu = if adres.trim().is_empty() {
            String::new()
        } else {
            format!(r##"<p class="sn-adres">{adres}</p>"##)
        };
        sahne_listesi.push((
            "saat",
            format!(
                r##"<section class="sn-sahne" data-sahne="4" aria-labelledby="sn-b4">
<div class="sn-sahne-ic sn-iki">
<p class="sn-sayi" aria-hidden="true">02</p>
<div>
<p class="sn-etiket">Ziyaret</p>
<h2 class="sn-h2" id="sn-b4">{bolge}</h2>
{saat_blogu}
{adres_blogu}
{yol_dugmesi}
</div>
</div>
</section>"##,
                yol_dugmesi = yol_baglantisi
                    .as_deref()
                    .map(|yol| format!(
                        r##"<a class="sn-buton kucuk" href="{yol}" target="_blank" rel="noopener">{KONUM_IKON}<span>Yol tarifi al</span></a>"##
                    ))
                    .unwrap_or_default(),
            ),
        ));
    }

    // 5 · iletişim
    let iletisim_satirlari = {
        let mut cikti = String::new();
        if let Some(hedef) = &tel_hedef {
            cikti.push_str(&format!(
                r##"<a class="sn-iletisim-satir" href="{hedef}">{TELEFON_IKON}<span><b>{tel_goster}</b><small>Hemen ara</small></span></a>"##
            ));
        }
        if !wa_no.is_empty() {
            cikti.push_str(&format!(
                r##"<a class="sn-iletisim-satir" href="https://wa.me/{wa_no}" target="_blank" rel="noopener"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" aria-hidden="true"><path d="M21 11.5a8.5 8.5 0 0 1-12.6 7.4L3 21l2.1-5.4A8.5 8.5 0 1 1 21 11.5z"/></svg><span><b>WhatsApp</b><small>Mesaj yaz</small></span></a>"##
            ));
        }
        if let Some(yol) = &yol_baglantisi {
            cikti.push_str(&format!(
                r##"<a class="sn-iletisim-satir" href="{yol}" target="_blank" rel="noopener">{KONUM_IKON}<span><b>Yol tarifi</b><small>Haritada aç</small></span></a>"##
            ));
        }
        cikti
    };
    sahne_listesi.push((
        "iletisim",
        format!(
            r##"<section class="sn-sahne" data-sahne="5" aria-labelledby="sn-b5">
<div class="sn-sahne-ic">
<p class="sn-etiket">İletişim</p>
<h2 class="sn-h2" id="sn-b5">{baslik}</h2>
<div class="sn-iletisim">{iletisim_satirlari}</div>
</div>
</section>"##,
            baslik = sn_satirlar(&format!("Bir sonraki adım,\n{ad}")),
        ),
    ));

    // Sahne sayısı: hero ve iletişim her zaman var; aradakiler sırayla.
    let oncelik = ["hero", "manifesto", "hizmet", "galeri", "saat", "iletisim"];
    let mut secilen: Vec<&str> = vec!["hero", "iletisim"];
    for adi in ["hizmet", "galeri", "manifesto", "saat"] {
        if secilen.len() >= sahne_sayisi {
            break;
        }
        if sahne_listesi.iter().any(|(kimlik, _)| *kimlik == adi) {
            secilen.push(adi);
        }
    }
    let mut sahneler = String::new();
    for adi in oncelik.iter() {
        if !secilen.contains(adi) {
            continue;
        }
        if let Some((_, html)) = sahne_listesi.iter().find(|(kimlik, _)| kimlik == adi) {
            sahneler.push_str(html);
        }
    }

    // Yapısal veri: yalnız kayıtta olan alanlar yazılır.
    let mut json = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "LocalBusiness",
        "name": &g.ad,
        "address": { "@type": "PostalAddress", "streetAddress": &g.adres, "addressCountry": "TR" },
    });
    if let Some(tel) = telefon {
        json["telephone"] = serde_json::Value::String(format!("+{tel}"));
    }
    if let Some((enlem, boylam)) = g.konum {
        json["geo"] = serde_json::json!({ "@type": "GeoCoordinates", "latitude": enlem, "longitude": boylam });
    }
    if let Some(puan) = g.puan.as_deref().filter(|p| !p.trim().is_empty()) {
        if let Ok(deger) = puan.replace(',', ".").parse::<f64>() {
            json["aggregateRating"] = serde_json::json!({
                "@type": "AggregateRating",
                "ratingValue": deger,
                "reviewCount": g
                    .yorum
                    .as_deref()
                    .and_then(|y| y.split_whitespace().next())
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(1),
            });
        }
    }
    let json_ld = serde_json::to_string(&json)
        .unwrap_or_default()
        .replace("</", "<\\/");

    let mobil_bar = {
        let mut parcalar = String::new();
        if let Some(hedef) = &tel_hedef {
            parcalar.push_str(&format!(
                r##"<a class="sn-bar-buton" href="{hedef}">{TELEFON_IKON}<span>Ara</span></a>"##
            ));
        }
        if !wa_no.is_empty() {
            parcalar.push_str(&format!(
                r##"<a class="sn-bar-buton dolu" href="https://wa.me/{wa_no}" target="_blank" rel="noopener"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="M21 11.5a8.5 8.5 0 0 1-12.6 7.4L3 21l2.1-5.4A8.5 8.5 0 1 1 21 11.5z"/></svg><span>WhatsApp</span></a>"##
            ));
        }
        if parcalar.is_empty() {
            String::new()
        } else {
            format!(
                r##"<div class="sn-bar" role="group" aria-label="Hızlı iletişim">{parcalar}</div>"##
            )
        }
    };

    let satici = match g.satici_tel.as_deref().map(str::trim) {
        Some(tel) if !tel.is_empty() => {
            let mesaj = url_kacis(&format!(
                "Merhaba, {} için hazırlanan demo taslağı hakkında bilgi almak istiyorum.",
                g.ad
            ));
            format!(
                r##"<section class="sn-bant" id="sn-satici"><p><b>{}</b> için hazırladığımız bu taslağı size özel hale getirebiliriz.</p><a class="sn-buton kucuk" href="https://wa.me/{tel}?text={mesaj}" target="_blank" rel="noopener">WhatsApp'tan yazın</a></section>"##,
                kacis(&g.gonderen)
            )
        }
        _ => String::new(),
    };

    let karne = g
        .karne
        .as_deref()
        .map(|karne| {
            format!(
                r##"<a class="sn-karne" href="{}">Dijital görünürlük karnesi</a>"##,
                kacis(karne)
            )
        })
        .unwrap_or_default();

    let program_js = g.program.as_deref().unwrap_or("null");

    // Prosedürel görsel dünya: işletme tohumundan türeyen renk/hız/partikül.
    let dunya = dunya_uret(&tohum, &vurgu, &zemin);
    let dunya_stil = format!(
        "--sn-akis1:{};--sn-akis2:{};--sn-hiz:{:.2};--sn-dalga:{:.2};--sn-parca:{:.2};--sn-vinyet:{:.2};--sn-aci:{};--sn-aci2:{}",
        dunya.akis1, dunya.akis2, dunya.hiz, dunya.dalga, dunya.parca, dunya.vinyet,
        dunya.aci, dunya.aci2
    );
    let mut sayfa = String::with_capacity(64 * 1024);
    sayfa.push_str("<!DOCTYPE html>\n<html lang=\"tr\">\n<head>\n<meta charset=\"utf-8\">\n");
    sayfa.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n");
    sayfa.push_str(&format!(
        "<title>{ad} — {sektor} | {bolge}</title>\n<meta name=\"description\" content=\"{} — {sektor}, {bolge}. {}.\">\n",
        kacis(&g.ad),
        kacis(&slogan)
    ));
    sayfa.push_str(&format!(
        "<meta property=\"og:title\" content=\"{ad} — {sektor}\"><meta property=\"og:description\" content=\"{}\"><meta property=\"og:type\" content=\"website\"><meta name=\"theme-color\" content=\"{zemin}\">\n",
        kacis(&slogan)
    ));
    sayfa.push_str("<script>document.documentElement.dataset.js=\"1\";</script>\n<style>");
    sayfa.push_str(g.font_css);
    sayfa.push_str(&format!(
        ":root{{--sn-vurgu:{vurgu};--sn-zemin:{zemin};--sn-yazi:{yazi};--sn-baslik:{baslik_yigin};--sn-govde:{govde_yigin}}}\n"
    ));
    sayfa.push_str(SINEMATIK_CSS);
    // Kart fotoğrafları: her görsel tek kez kodlanır.
    sayfa.push_str(&kart_foto_kurallari(g));
    let medya_var = if g.video.is_some() || g.foto.is_some() {
        "1"
    } else {
        "0"
    };
    sayfa.push_str(&format!(
        "</style>\n</head>\n<body class=\"sn-govde\" data-profil=\"sinematik\" data-medya=\"{medya_var}\" data-zemin=\"{zemin_modu}\">\n"
    ));
    // Kayan yazı şeridi: aynı grup dört kez, kesintisiz döngü için.
    let seri =
        format!(r##"<span>{sektor}<i>·</i>{ad}<i>·</i>{bolge}<i>·</i>{cta}<i>·</i></span>"##);
    let marlee = format!(
        r##"<div class="sn-serif" aria-hidden="true"><div class="sn-serif-ic">{seri}{seri}{seri}{seri}</div></div>"##
    );

    sayfa.push_str(&format!(
        r##"<div class="sn-perde" id="sn-perde" aria-hidden="true"><div class="sn-perde-ic"><span class="sn-perde-ad">{ad}</span><span class="sn-perde-yuzde" id="sn-perde-yuzde">0%</span><i class="sn-perde-cizgi"></i></div></div>
<a class="sn-atla" href="#sn-icerik">İçeriğe geç</a>
<div class="sn-ilerleme" aria-hidden="true"><i></i></div>
<canvas class="sn-tuval" aria-hidden="true"></canvas>
<div class="sn-gren" aria-hidden="true"></div>
<div class="sn-vinyet" aria-hidden="true"></div>
{zemin_katmani}
<div class="sn-imlec" aria-hidden="true"></div>
<div class="sn-imlec-halka" aria-hidden="true"></div>
<header class="sn-ust">
<a class="sn-marka" href="#sn-icerik">{ad}</a>
<nav class="sn-menu" aria-label="Bölümler"><a href="#sn-b2">Hizmetler</a><a href="#sn-b3">Galeri</a><a href="#sn-b5">İletişim</a></nav>
{ust_cta}
</header>
<main id="sn-icerik" class="sn-akis" data-stil="{stil}" data-nesne="{nesne}" data-hareket="{hareket}" data-tohum="{tohum}" data-dunya="{dunya_tohum}" style="{dunya_stil}">{sahneler}</main>
{marlee}
{satici}
<footer class="sn-alt"><div><b>{ad}</b><span>{bolge_satiri}</span>{video_atif}</div><div class="sn-alt-baglar">{karne}<span>Demo taslağı · {gonderen}</span></div></footer>
{mobil_bar}
<script>var PROGRAM={program_js};</script>
<script>{js}</script>
</body>
</html>
"##,
        zemin_katmani = zemin_katmani,
        ust_cta = tel_hedef
            .as_deref()
            .map(|hedef| format!(
                r##"<a class="sn-ust-cta" href="{hedef}">{TELEFON_IKON}<span>{tel_goster}</span></a>"##
            ))
            .unwrap_or_default(),
        gonderen = kacis(&g.gonderen),
        video_atif = g
            .video
            .as_ref()
            .and_then(|video| video.atif.as_deref())
            .map(|atif| format!(
                r##"<span class="sn-video-atıf">Görsel: {}</span>"##,
                kacis(atif)
            ))
            .unwrap_or_default(),
        js = SINEMATIK_JS.replace("__PROGRAM__", program_js),
        dunya_tohum = dunya.tohum,
        dunya_stil = dunya_stil,
    ));
    sayfa.push_str(&format!(
        "<script type=\"application/ld+json\">{json_ld}</script>\n"
    ));
    sayfa
}

/// Bütçe denetimli üretim: sinematik sayfa 1,35 MB'ı aşarsa galeri kareleri
/// sondan başlayarak düşürülür. Döndürülen sayı, çıkarılan kare sayısıdır.
pub fn uret_butceli(g: &Girdi, butce: usize) -> (String, usize) {
    if !g.profil.eq_ignore_ascii_case("sinematik") {
        return (kucult(&uret(g)), 0);
    }
    let mut aday = g.clone();
    let mut kirpilan = 0usize;
    loop {
        let html = kucult(&uret(&aday));
        if html.len() <= butce || aday.galeri.is_empty() {
            return (html, kirpilan);
        }
        aday.galeri.pop();
        kirpilan += 1;
    }
}

const SINEMATIK_CSS: &str = r##"
*,*::before,*::after{box-sizing:border-box}
html{scroll-behavior:smooth}
body{margin:0;background:var(--sn-zemin);color:var(--sn-yazi);font-family:var(--sn-govde);-webkit-font-smoothing:antialiased;overflow-x:hidden}
a{color:inherit}
.sn-atla{position:absolute;left:-9999px;top:0;z-index:99;background:var(--sn-vurgu);color:#08080a;padding:10px 16px;border-radius:0 0 10px 0;font-size:13px}
.sn-atla:focus{left:0}
.sn-ilerleme{position:fixed;left:0;top:0;height:2px;width:100%;z-index:8;background:color-mix(in srgb,var(--sn-yazi) 12%,transparent)}
.sn-ilerleme i{display:block;height:100%;width:var(--sn-kaydirma,0%);background:var(--sn-vurgu);transition:width .1s linear}
.sn-ust{position:fixed;top:10px;left:clamp(10px,2vw,24px);right:clamp(10px,2vw,24px);z-index:7;display:flex;align-items:center;justify-content:space-between;gap:14px;padding:11px clamp(14px,2vw,22px);border:1px solid color-mix(in srgb,var(--sn-yazi) 14%,transparent);border-radius:16px;background:color-mix(in srgb,var(--sn-zemin) 58%,transparent);-webkit-backdrop-filter:blur(22px) saturate(1.4);backdrop-filter:blur(22px) saturate(1.4);box-shadow:0 18px 50px -30px rgba(0,0,0,.9)}
.sn-marka{font-family:var(--sn-baslik);font-size:15px;letter-spacing:-.01em;text-decoration:none;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.sn-menu{display:flex;gap:22px;font-size:11.5px;letter-spacing:.14em;text-transform:uppercase}
.sn-menu a{text-decoration:none;opacity:.62;transition:opacity .25s ease}
.sn-menu a:hover{opacity:1}
.sn-ust-cta{display:inline-flex;align-items:center;gap:8px;text-decoration:none;font-size:12px;padding:8px 14px;border:1px solid color-mix(in srgb,var(--sn-vurgu) 55%,transparent);border-radius:999px;color:var(--sn-vurgu);transition:background .3s ease,color .3s ease}
.sn-ust-cta:hover{background:var(--sn-vurgu);color:#08080a}
.sn-ust-cta svg{width:14px;height:14px}
.sn-tuval{position:fixed;inset:0;width:100%;height:100%;z-index:0;pointer-events:none;opacity:.55;display:block}
html[data-tuval="yok"] .sn-tuval,html[data-tuval="zayif"] .sn-tuval{display:none}
.sn-gren{position:fixed;inset:0;z-index:1;pointer-events:none;opacity:.14;background-image:url("data:image/svg+xml;utf8,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120'%3E%3Cfilter id='g'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='2'/%3E%3C/filter%3E%3Crect width='120' height='120' filter='url(%23g)' opacity='.5'/%3E%3C/svg%3E")}
.sn-akis{position:relative;z-index:2}
/* Aurora: işletme tohumundan gelen iki renk, hız ve yoğunluk. */
.sn-akis::before{content:"";position:fixed;inset:-18%;z-index:-1;pointer-events:none;background:radial-gradient(42% 42% at 24% 28%,color-mix(in srgb,var(--sn-akis1,var(--sn-vurgu)) 62%,transparent),transparent 70%),radial-gradient(48% 48% at 76% 66%,color-mix(in srgb,var(--sn-akis2,var(--sn-vurgu)) 52%,transparent),transparent 72%),radial-gradient(38% 38% at 52% 88%,color-mix(in srgb,var(--sn-akis1,var(--sn-vurgu)) 30%,transparent),transparent 74%);filter:blur(46px);opacity:calc(.28 + var(--sn-parca,.35)*.4);animation:snAkis calc(26s / var(--sn-hiz,1)) ease-in-out infinite alternate;will-change:transform}
html[data-hareket="statik"] .sn-akis::before{animation:none}
.sn-sahne::before{background-image:radial-gradient(35% 40% at calc(10% + var(--sn-aci,0) * .2%) 8%,color-mix(in srgb,var(--sn-akis1,var(--sn-vurgu)) 16%,transparent),transparent 62%)}
.sn-sahne{position:relative;min-height:100vh;min-height:100svh;display:grid;align-items:center;padding:clamp(84px,11vw,150px) clamp(18px,5vw,64px);border-bottom:1px solid color-mix(in srgb,var(--sn-yazi) 9%,transparent);overflow:hidden}
.sn-sahne::before{content:"";position:absolute;inset:0;z-index:-1;background:radial-gradient(120% 90% at 18% 0%,color-mix(in srgb,var(--sn-vurgu) 10%,transparent),transparent 62%),var(--sn-zemin)}
html[data-js="1"] .sn-sahne{position:sticky;top:0;height:100vh;height:100svh;min-height:0;border-bottom:0}
.sn-sahne-ic{width:100%;max-width:1180px;margin:0 auto;position:relative;transform:translate3d(0,calc(var(--sn-p,0)*-38px),0);opacity:calc(1 - var(--sn-p,0)*.8)}
html[data-hareket="statik"] .sn-sahne{position:relative;height:auto;min-height:100svh}
html[data-hareket="statik"] .sn-sahne-ic{transform:none;opacity:1}
/* Full-bleed hero: medya tüm sahneyi kaplar, içerik üstüne biner.
   Bulanıklık/sis yok; yalnız metin arkasında akışkan gradyan vinyet. */
.sn-hero{position:relative;overflow:hidden;align-items:end;padding-bottom:clamp(96px,10vh,150px)}
/* Tam ekran sabit video: klip tüm sayfanın arkasında durur, içerik
   üzerinden akar. Sahneler sticky yığınını bırakır ki birbirini örtmesin. */
.sn-zemin-medya{position:fixed;inset:0;z-index:0;overflow:hidden;background:#000;pointer-events:none}
.sn-zemin-medya .sn-hero-foto{position:absolute;inset:0;width:100%;height:100%;margin:0;border-radius:0;box-shadow:none;transform:translate3d(0,calc(var(--sn-par,0) * -18px),0) scale(1.05);overflow:hidden;background-color:#000;background-image:var(--sn-poster);background-size:cover;background-position:50% 42%}
.sn-zemin-medya .sn-hero-foto img{width:100%;height:100%;object-fit:cover;object-position:var(--sn-v-konum,50% 42%)}
.sn-zemin-medya .sn-hero-video{aspect-ratio:auto;max-height:none;border-radius:0;box-shadow:none}
.sn-zemin-medya .sn-video{width:100%;height:100%;object-fit:cover;object-position:var(--sn-v-konum,50% 50%)}
.sn-zemin-medya .sn-karart{display:none}
.sn-zemin-perde{position:absolute;inset:0;z-index:1;pointer-events:none;background:linear-gradient(180deg,rgba(0,0,0,.44),rgba(0,0,0,0) 22%,rgba(0,0,0,0) 62%,rgba(0,0,0,.52)),radial-gradient(130% 100% at 50% 46%,transparent 36%,rgba(0,0,0,.44))}
/* Hero cam paneli: tam ekran video arka planının üzerinde, metni ve
   çağrıyı tek bir yarı saydam yüzeyde toplar. Arka plan videosu kesintisiz
   akarken panel okunabilirliği blur + ince ışık kenarıyla korur. */
body[data-zemin="medya"] .sn-hero .sn-sahne-ic{width:min(100%,1040px);padding:clamp(28px,4.4vw,58px);border:1px solid rgba(255,255,255,.22);border-radius:30px;background:linear-gradient(135deg,rgba(13,17,24,.54),rgba(13,17,24,.20));-webkit-backdrop-filter:blur(22px) saturate(135%);backdrop-filter:blur(22px) saturate(135%);box-shadow:0 30px 90px rgba(0,0,0,.42),inset 0 1px 0 rgba(255,255,255,.24)}
body[data-zemin="medya"] .sn-hero .sn-sahne-ic::before{inset:-2% -2%;background:linear-gradient(90deg,rgba(0,0,0,.52),transparent 70%);border-radius:inherit;z-index:-1}
body[data-zemin="medya"] .sn-hero .sn-hero-metin{padding-top:clamp(16px,4vh,42px)}
@media(max-width:760px){body[data-zemin="medya"] .sn-hero .sn-sahne-ic{padding:22px 18px;border-radius:22px;background:linear-gradient(135deg,rgba(13,17,24,.64),rgba(13,17,24,.32));-webkit-backdrop-filter:blur(14px) saturate(125%);backdrop-filter:blur(14px) saturate(125%)}body[data-zemin="medya"] .sn-hero .sn-hero-metin{padding-top:18px}}
body[data-zemin="medya"] .sn-sahne{position:relative;height:auto;min-height:100vh;min-height:100svh;overflow:hidden;border-bottom:1px solid color-mix(in srgb,var(--sn-yazi) 8%,transparent)}
body[data-zemin="medya"] .sn-sahne::before{background:radial-gradient(60% 60% at 16% 0%,color-mix(in srgb,var(--sn-vurgu) 14%,transparent),transparent 68%)}
body[data-zemin="medya"] .sn-sahne::after{display:none}
body[data-zemin="medya"] .sn-sahne-ic{transform:none;opacity:1;isolation:isolate}
body[data-zemin="medya"] .sn-sahne-ic::before{content:"";position:absolute;inset:-6% -8%;z-index:-1;pointer-events:none;border-radius:44px;background:radial-gradient(112% 124% at 2% 62%,rgba(0,0,0,.66),rgba(0,0,0,.30) 46%,transparent 74%)}
body[data-zemin="medya"] .sn-hero .sn-sahne-ic::before{inset:-14% -18%;background:radial-gradient(88% 110% at 10% 66%,rgba(0,0,0,.74),rgba(0,0,0,.36) 46%,transparent 76%)}
body[data-zemin="medya"] .sn-kart{background:color-mix(in srgb,var(--sn-zemin) 48%,transparent);-webkit-backdrop-filter:blur(10px) saturate(1.1);backdrop-filter:blur(10px) saturate(1.1)}
body[data-zemin="medya"] .sn-bant{background:color-mix(in srgb,var(--sn-zemin) 62%,transparent);-webkit-backdrop-filter:blur(12px);backdrop-filter:blur(12px)}
body[data-zemin="medya"] .sn-h1,body[data-zemin="medya"] .sn-h2,body[data-zemin="medya"] .sn-lede,body[data-zemin="medya"] .sn-metin,body[data-zemin="medya"] .sn-etiket,body[data-zemin="medya"] .sn-sayi{text-shadow:0 3px 26px rgba(0,0,0,.82),0 1px 2px rgba(0,0,0,.6)}
body[data-zemin="medya"] .sn-hero{min-height:100vh;min-height:100svh}
/* Tam ekran arka plan videosu: sayfa yüksekliğine değil görünür alana
   oturur; object-fit cover ile oranı bozmadan kırpar. */
.bg-video{position:fixed;inset:0;width:100vw;height:100vh;min-width:100%;min-height:100%;object-fit:cover;object-position:50% 50%;z-index:0;pointer-events:none}
/* Hero içeriği 40–50% koyulukta cam panel: video arkada kesintisiz akar. */
body[data-zemin="medya"] .sn-hero .sn-sahne-ic.hero-overlay{background:rgba(8,11,15,.46);-webkit-backdrop-filter:blur(4px) saturate(115%);backdrop-filter:blur(4px) saturate(115%);border-color:rgba(255,255,255,.18)}
@media(max-width:760px){body[data-zemin="medya"] .sn-hero .sn-sahne-ic.hero-overlay{background:rgba(8,11,15,.48);-webkit-backdrop-filter:blur(4px);backdrop-filter:blur(4px)}}
.sn-hero-metin{position:relative;z-index:2;max-width:min(860px,100%);padding:clamp(88px,13vh,170px) 0 0;text-shadow:0 2px 30px rgba(0,0,0,.74)}
.sn-hero-metin .sn-lede{max-width:44ch}
.sn-video{display:block;width:100%;height:100%;object-fit:cover;object-position:var(--sn-v-konum,50% 50%)}
/* Arka plan sözleşmesi: video kabı ne olursa olsun görünür alanı kaplar. */
.sn-video.bg-video{position:fixed;inset:0;width:100vw;height:100vh;object-fit:cover}
html[data-video="yok"] .sn-video{opacity:0}
/* --- Ken Burns + paralaks: her fotoğraf akar, kaydırmayla derinlik alır --- */
.sn-kare img,.sn-hero-foto img,.sn-kart-foto{animation:snKenBurns var(--kb-s,16s) cubic-bezier(.4,0,.2,1) infinite alternate;animation-delay:var(--kb-d,0s);transform-origin:var(--kb-ox,50%) var(--kb-oy,50%);will-change:transform}
.sn-kare,.sn-kart-foto,.sn-hero-foto{transform:translate3d(0,calc(var(--sn-par,0) * -16px),0);transition:transform .5s cubic-bezier(.2,.8,.2,1)}
.sn-kare:hover,.sn-hero-foto:hover{transform:translate3d(0,calc(var(--sn-par,0) * -16px),0) scale(1.03)}
.sn-kare:hover img,.sn-hero-foto:hover img,.sn-kart-foto:hover img{animation-duration:calc(var(--kb-s,16s) * .5)}
.sn-kart-foto{display:block;margin:0 0 16px;border-radius:12px;overflow:hidden;aspect-ratio:16/10;border:1px solid color-mix(in srgb,var(--sn-yazi) 10%,transparent);background-size:cover;background-position:50% 50%;filter:saturate(.92) contrast(1.06)}
body[data-medya="1"] .sn-tuval,body[data-medya="1"] .sn-akis::before{display:none}
@keyframes snKenBurns{from{transform:scale(1.02) translate3d(0,0,0)}to{transform:scale(var(--kb-z,1.16)) translate3d(var(--kb-x,0%),var(--kb-y,0%),0)}}
html[data-hareket="statik"] .sn-kare img,html[data-hareket="statik"] .sn-hero-foto img,html[data-hareket="statik"] .sn-kart-foto{animation:none}
.sn-etiket{display:flex;gap:14px;align-items:center;font-size:11px;letter-spacing:.2em;text-transform:uppercase;opacity:.66;margin:0 0 18px}
.sn-h1{font-family:var(--sn-baslik);font-size:clamp(2.7rem,7.4vw,6.6rem);line-height:.93;letter-spacing:-.05em;font-weight:600;margin:0 0 22px;max-width:16ch}
.sn-h2{font-family:var(--sn-baslik);font-size:clamp(2rem,4.6vw,3.9rem);line-height:1;letter-spacing:-.045em;font-weight:600;margin:0 0 18px;max-width:22ch}
.sn-satir{display:block;overflow:hidden;padding:0 0 .08em}
.sn-harf-kap{display:block}
.sn-h{display:inline-block}
html[data-js="1"] .sn-h{transform:translateY(118%) rotate(3deg);opacity:0;animation:snHarf .92s cubic-bezier(.16,.84,.28,1) forwards;animation-delay:calc(var(--g,0)*1ms + var(--i,0)*24ms)}
html[data-js="1"] .sn-h1{transform:scale(calc(1 - var(--sn-p,0)*.055));opacity:calc(1 - var(--sn-p,0)*.6);transform-origin:0 50%;will-change:transform,opacity}
.sn-ekran{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;border:0}
html[data-hareket="statik"] .sn-h{transform:none;opacity:1;animation:none}
html[data-hareket="statik"] .sn-h1{transform:none;opacity:1}
.sn-lede{font-size:clamp(15px,1.5vw,19px);line-height:1.6;max-width:46ch;opacity:.76;margin:0 0 30px}
.sn-metin{font-size:clamp(14px,1.3vw,17px);line-height:1.7;opacity:.74;max-width:58ch;margin:0}
.sn-butonlar{display:flex;flex-wrap:wrap;gap:12px}
.sn-buton{display:inline-flex;align-items:center;gap:9px;padding:14px 22px;border-radius:999px;text-decoration:none;font-size:13px;border:1px solid color-mix(in srgb,var(--sn-yazi) 26%,transparent);transition:transform .3s cubic-bezier(.2,.8,.2,1),box-shadow .3s ease,border-color .3s ease,background .3s ease}
.sn-buton svg{width:15px;height:15px}
.sn-buton:hover{transform:translateY(-3px);border-color:var(--sn-vurgu);box-shadow:0 18px 44px -20px var(--sn-vurgu)}
.sn-buton.dolu{background:var(--sn-vurgu);border-color:var(--sn-vurgu);color:#08080a;font-weight:600}
.sn-buton.dolu:hover{box-shadow:0 22px 60px -18px var(--sn-vurgu)}
.sn-buton.kucuk{padding:10px 16px;font-size:12px;margin-top:18px}
/* Sinematik spotlight: video sağ yarıyı domine eder, geniş ekranda ekran
   kenarına taşar (full-bleed). Sol sütun metne daralır. */
.sn-hero-grid{display:block;width:100%;min-height:0}
.sn-hero-metin{position:relative;z-index:2}
.sn-hero-sahne{position:relative;display:grid;place-items:center;min-height:min(66vh,640px);width:100%}
.sn-hero-foto{margin:0;width:100%;border-radius:18px;overflow:hidden;box-shadow:0 40px 120px -50px var(--sn-vurgu)}
.sn-hero-foto img{display:block;width:100%;height:min(62vh,560px);object-fit:cover}
.sn-durum{display:inline-flex;align-items:center;gap:7px;padding:5px 11px;border-radius:999px;font-size:10.5px;letter-spacing:.08em;border:1px solid color-mix(in srgb,var(--sn-yazi) 22%,transparent);text-transform:uppercase}
.sn-durum::before{content:"";width:6px;height:6px;border-radius:50%;background:#d0554a}
.sn-durum.acik::before{background:#4ec98a;box-shadow:0 0 12px #4ec98a}
.sn-asagi{position:absolute;left:50%;bottom:26px;transform:translateX(-50%);width:22px;height:36px;border:1px solid color-mix(in srgb,var(--sn-yazi) 30%,transparent);border-radius:999px;display:block;z-index:3}
.sn-asagi i{position:absolute;left:50%;top:8px;width:3px;height:8px;margin-left:-1.5px;border-radius:2px;background:var(--sn-vurgu);animation:snIn 2.1s ease-in-out infinite}
.sn-sayi{font-family:var(--sn-baslik);font-size:clamp(3rem,9vw,8rem);line-height:.8;margin:0;opacity:.14;letter-spacing:-.06em}
.sn-iki{display:grid;grid-template-columns:auto minmax(0,1fr);gap:clamp(20px,4vw,60px);align-items:start}
.sn-kartlar{list-style:none;display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:16px;padding:0;margin:30px 0 0}
.sn-kart{position:relative;padding:26px 22px;border:1px solid color-mix(in srgb,var(--sn-yazi) 14%,transparent);border-radius:16px;background:color-mix(in srgb,var(--sn-zemin) 72%,transparent);-webkit-backdrop-filter:blur(14px);backdrop-filter:blur(14px);transform-style:preserve-3d;transition:transform .35s cubic-bezier(.2,.8,.2,1),box-shadow .35s ease,border-color .35s ease}
.sn-kart::after{content:"";position:absolute;inset:0;border-radius:inherit;pointer-events:none;opacity:0;background:radial-gradient(60% 60% at var(--sn-mx,50%) var(--sn-my,50%),color-mix(in srgb,var(--sn-vurgu) 34%,transparent),transparent 70%);transition:opacity .35s ease}
.sn-kart:hover{transform:translateY(-8px);border-color:color-mix(in srgb,var(--sn-vurgu) 62%,transparent);box-shadow:0 26px 70px -34px var(--sn-vurgu)}
.sn-kart:hover::after{opacity:1}
.sn-kart-no{font-size:11px;letter-spacing:.18em;opacity:.5}
.sn-kart h3{font-size:17px;margin:10px 0 8px;letter-spacing:-.01em}
.sn-kart p{margin:0;font-size:13px;line-height:1.6;opacity:.7}
.sn-galeri{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:14px;margin-top:26px}
.sn-kare{margin:0;border-radius:14px;overflow:hidden;border:1px solid color-mix(in srgb,var(--sn-yazi) 12%,transparent)}
.sn-kare img{display:block;width:100%;height:clamp(200px,34vh,360px);object-fit:cover}
.sn-saatler{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:2px 22px;margin:16px 0 20px;max-width:620px}
.sn-saat-satir{display:flex;justify-content:space-between;gap:12px;font-size:12.5px;padding:7px 0;border-bottom:1px solid color-mix(in srgb,var(--sn-yazi) 9%,transparent)}
.sn-saat-satir span{opacity:.6}
.sn-adres{font-size:13px;opacity:.68;max-width:46ch;margin:0}
.sn-iletisim{display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:12px;margin-top:24px}
.sn-iletisim-satir{display:flex;align-items:center;gap:14px;padding:18px;border-radius:14px;text-decoration:none;border:1px solid color-mix(in srgb,var(--sn-yazi) 14%,transparent);transition:border-color .3s ease,transform .3s ease}
.sn-iletisim-satir:hover{border-color:var(--sn-vurgu);transform:translateY(-4px)}
.sn-iletisim-satir svg{width:20px;height:20px;color:var(--sn-vurgu);flex:0 0 auto}
.sn-iletisim-satir b{display:block;font-size:13.5px}
.sn-iletisim-satir small{display:block;font-size:11px;opacity:.55;margin-top:3px}
.sn-nesne{position:relative;width:min(78%,420px);aspect-ratio:1;display:grid;place-items:center;color:var(--sn-vurgu)}
.sn-halka{transform-style:preserve-3d;animation:snDon 26s linear infinite}
.sn-halka i{position:absolute;inset:26%;border:1px solid color-mix(in srgb,var(--sn-vurgu) 55%,transparent);border-radius:14px;background:color-mix(in srgb,var(--sn-vurgu) 7%,transparent);transform:rotateY(calc(var(--i)*45deg)) translateZ(150px);-webkit-backdrop-filter:blur(4px);backdrop-filter:blur(4px)}
.sn-halka b{position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);font-family:var(--sn-baslik);font-size:24px;letter-spacing:-.02em;color:var(--sn-yazi);opacity:.75;text-align:center;width:60%}
.sn-kup{transform-style:preserve-3d;animation:snDon 22s linear infinite}
.sn-kup i{position:absolute;inset:24%;border:1px solid color-mix(in srgb,var(--sn-vurgu) 58%,transparent);background:color-mix(in srgb,var(--sn-vurgu) 6%,transparent)}
.sn-kup .on{transform:translateZ(90px)}
.sn-kup .arka{transform:rotateY(180deg) translateZ(90px)}
.sn-kup .sag{transform:rotateY(90deg) translateZ(90px)}
.sn-kup .sol{transform:rotateY(-90deg) translateZ(90px)}
.sn-kup .ust{transform:rotateX(90deg) translateZ(90px)}
.sn-kup .alt{transform:rotateX(-90deg) translateZ(90px)}
.sn-geometri svg{width:100%;height:auto;opacity:.8;animation:snNefes 9s ease-in-out infinite}
.sn-dalga-kutu{width:min(100%,460px)}
.sn-dalga{width:100%;height:auto;display:block;opacity:.85}
.sn-bant{position:relative;z-index:2;display:flex;flex-wrap:wrap;gap:14px;align-items:center;justify-content:space-between;padding:26px clamp(18px,5vw,64px);border-top:1px solid color-mix(in srgb,var(--sn-yazi) 14%,transparent);background:color-mix(in srgb,var(--sn-vurgu) 8%,var(--sn-zemin))}
.sn-bant p{margin:0;font-size:13px;opacity:.8;max-width:60ch}
.sn-alt{position:relative;z-index:2;display:flex;flex-wrap:wrap;gap:10px;justify-content:space-between;padding:26px clamp(18px,5vw,64px) 90px;font-size:11.5px;opacity:.7;border-top:1px solid color-mix(in srgb,var(--sn-yazi) 10%,transparent)}
.sn-alt b{display:block;font-size:13px;opacity:.9}
.sn-alt-baglar{display:flex;flex-direction:column;gap:6px;align-items:flex-end;text-align:right}
.sn-karne{text-decoration:underline;text-underline-offset:3px}
.sn-bar{position:fixed;left:0;right:0;bottom:0;z-index:9;display:none;gap:8px;padding:10px 12px;background:color-mix(in srgb,var(--sn-zemin) 88%,transparent);-webkit-backdrop-filter:blur(16px);backdrop-filter:blur(16px);border-top:1px solid color-mix(in srgb,var(--sn-yazi) 14%,transparent);transition:transform .35s cubic-bezier(.2,.8,.2,1)}
.sn-bar.gizli{transform:translateY(115%)}
.sn-bar-buton{flex:1;display:inline-flex;align-items:center;justify-content:center;gap:8px;padding:12px;border-radius:10px;text-decoration:none;font-size:12.5px;border:1px solid color-mix(in srgb,var(--sn-yazi) 20%,transparent)}
.sn-bar-buton.dolu{background:var(--sn-vurgu);border-color:var(--sn-vurgu);color:#08080a;font-weight:600}
.sn-bar-buton svg{width:16px;height:16px}
/* --- Premium katman: perde, vinyet, imleç, şerit, sahne geçişi ---------- */
.sn-perde{position:fixed;inset:0;z-index:60;display:none;place-items:center;background:var(--sn-zemin)}
html[data-js="1"] .sn-perde{display:grid}
.sn-perde-ic{display:grid;justify-items:center;gap:12px;text-align:center;padding:24px}
.sn-perde-ad{font-family:var(--sn-baslik);font-size:clamp(1.4rem,4vw,2.2rem);letter-spacing:-.03em}
.sn-perde-yuzde{font-size:11px;letter-spacing:.28em;opacity:.62;font-variant-numeric:tabular-nums}
.sn-perde-cizgi{display:block;width:min(220px,52vw);height:1px;background:color-mix(in srgb,var(--sn-yazi) 18%,transparent);position:relative;overflow:hidden}
.sn-perde-cizgi::after{content:"";position:absolute;inset:0;background:var(--sn-vurgu);transform:scaleX(var(--sn-yuk,0));transform-origin:0 50%;transition:transform .2s linear}
.sn-perde.bitti{clip-path:inset(0 0 100% 0);transition:clip-path .85s cubic-bezier(.76,0,.24,1)}
.sn-perde.bitti .sn-perde-ic{opacity:0;transition:opacity .3s ease}
html[data-hareket="statik"] .sn-perde{display:none}
.sn-vinyet{position:fixed;inset:0;z-index:1;pointer-events:none;background:radial-gradient(calc(88% + var(--sn-vinyet,.5) * 44%) 100% at 50% 38%,transparent calc(30% + var(--sn-vinyet,.5) * 22%),rgba(0,0,0,var(--sn-vinyet,.5)) 100%)}
.sn-sahne::after{content:"";position:absolute;inset:0;z-index:3;pointer-events:none;background:var(--sn-zemin);opacity:min(.94,max(0,calc((var(--sn-p,0) - .58)*2.3)))}
html[data-hareket="statik"] .sn-sahne::after{display:none}
.sn-imlec,.sn-imlec-halka{position:fixed;left:0;top:0;z-index:70;pointer-events:none;border-radius:50%;opacity:0;transition:opacity .3s ease}
.sn-imlec{width:6px;height:6px;margin:-3px 0 0 -3px;background:var(--sn-vurgu);transition:opacity .3s ease,width .25s ease,height .25s ease,margin .25s ease}
.sn-imlec-halka{width:34px;height:34px;margin:-17px 0 0 -17px;border:1px solid color-mix(in srgb,var(--sn-vurgu) 60%,transparent)}
.sn-imlec.gorunur,.sn-imlec-halka.gorunur{opacity:1}
.sn-imlec.basili{width:16px;height:16px;margin:-8px 0 0 -8px}
html[data-imlec="1"],html[data-imlec="1"] a,html[data-imlec="1"] button{cursor:none}
html[data-hareket="statik"] .sn-imlec,html[data-hareket="statik"] .sn-imlec-halka{display:none}
.sn-serif{position:relative;z-index:4;overflow:hidden;padding:16px 0;border-top:1px solid color-mix(in srgb,var(--sn-yazi) 12%,transparent);border-bottom:1px solid color-mix(in srgb,var(--sn-yazi) 12%,transparent);background:color-mix(in srgb,var(--sn-vurgu) 7%,var(--sn-zemin))}
.sn-serif-ic{display:flex;gap:34px;width:max-content;animation:snKay 28s linear infinite;font-family:var(--sn-baslik);font-size:clamp(1.05rem,2.4vw,1.6rem);letter-spacing:.02em;white-space:nowrap;opacity:.82}
.sn-serif-ic span{display:inline-flex;align-items:center;gap:34px}
.sn-serif-ic i{font-style:normal;color:var(--sn-vurgu)}
html[data-hareket="statik"] .sn-serif-ic{animation:none}
html[data-hareket="statik"] .sn-serif{overflow-x:auto}
@keyframes snAkis{0%{transform:translate3d(-3%,-2%,0) scale(1.02) rotate(-2deg)}50%{transform:translate3d(2%,1%,0) scale(1.07) rotate(1deg)}100%{transform:translate3d(4%,-3%,0) scale(1.03) rotate(2deg)}}
@keyframes snHarf{to{transform:translateY(0) rotate(0);opacity:1}}
@keyframes snKay{to{transform:translateX(-50%)}}
@keyframes snDon{to{transform:rotateY(360deg) rotateX(12deg)}}
@keyframes snIn{0%,100%{transform:translateY(0);opacity:1}50%{transform:translateY(10px);opacity:.2}}
@keyframes snNefes{0%,100%{opacity:.55;transform:scale(.98)}50%{opacity:.9;transform:scale(1.02)}}
@media (max-width:820px){
.sn-menu{display:none}
.sn-hero-grid{grid-template-columns:1fr;gap:26px}
.sn-hero-sahne{min-height:auto;order:2}
.sn-hero-foto img{height:38vh}
.sn-hero-video{aspect-ratio:4/3;border-radius:18px;width:100%;margin-right:0}
.sn-nesne{width:min(70%,260px)}
.sn-iki{grid-template-columns:1fr}
.sn-sahne{padding:76px 18px}
html[data-js="1"] .sn-sahne{position:relative;height:auto;min-height:auto}
/* Mobilde de hero tam ekran kalsın: medya tüm görünür alanı kaplar. */
html[data-js="1"] .sn-hero{min-height:100vh;min-height:100svh}
html[data-js="1"] .sn-sahne-ic{transform:none;opacity:1}
.sn-bar{display:flex}
.sn-alt{padding-bottom:96px}
.sn-h1{font-size:clamp(2.3rem,10.5vw,3.4rem);overflow-wrap:anywhere}
.sn-h2{overflow-wrap:anywhere}
.sn-buton,.sn-ust-cta{min-height:46px;padding:13px 20px}
.sn-bar-buton{min-height:50px;padding:15px 12px}
.sn-bar{gap:10px;padding:12px}
.sn-serif{padding:12px 0}
}
@media (prefers-reduced-motion:reduce){
*,*::before,*::after{animation:none!important;transition:none!important}
.sn-satir>span{transform:none}
.sn-sahne-ic{transform:none;opacity:1}
}
"##;

const SINEMATIK_JS: &str = r##"
(function(){
  var K=document.documentElement;
  K.dataset.js="1";
  var azalt=window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var statik=azalt;
  if(azalt){K.dataset.hareket="statik";}
  var sahneler=[].slice.call(document.querySelectorAll(".sn-sahne"));
  var medyalar=[].slice.call(document.querySelectorAll(".sn-kare,.sn-kart-foto,.sn-hero-foto"));
  var vh=window.innerHeight||1;
  function olc(){vh=window.innerHeight||1;}
  addEventListener("resize",olc,{passive:true});
  var bar=document.querySelector(".sn-bar"), bant=document.getElementById("sn-satici");
  if(bar&&bant&&"IntersectionObserver" in window){
    new IntersectionObserver(function(g){bar.classList.toggle("gizli",g[0].isIntersecting);},{threshold:.04}).observe(bant);
  }
  function guncelle(){
    if(statik)return;
    var y=window.pageYOffset||document.documentElement.scrollTop;
    for(var i=0;i<sahneler.length;i++){
      var p=(y-i*vh)/vh;
      p=p<0?0:p>1?1:p;
      sahneler[i].style.setProperty("--sn-p",p.toFixed(3));
    }
    for(var m=0;m<medyalar.length;m++){
      var r=medyalar[m].getBoundingClientRect();
      var q=(r.top+r.height*.5-vh*.5)/vh;
      q=q<-1?-1:q>1?1:q;
      medyalar[m].style.setProperty("--sn-par",q.toFixed(3));
    }
    var toplam=K.scrollHeight-vh;
    K.style.setProperty("--sn-kaydirma",(toplam>0?Math.min(100,y/toplam*100):0).toFixed(2)+"%");
  }
  var bekliyor=false;
  addEventListener("scroll",function(){
    if(bekliyor)return;
    bekliyor=true;
    requestAnimationFrame(function(){bekliyor=false;guncelle();});
  },{passive:true});
  addEventListener("scrollend",guncelle,{passive:true});
  guncelle();

  /* Hero videosu: sessiz, otomatik ve kesintisiz döngü. Poster ilk kare
     olarak durur; video hazır olunca başlar. Tarayıcı otomatik oynatmayı
     engellerse ilk dokunuşta devreye girer, sekme geri gelince devam eder.
     Döngü bitişinde (loop desteklenmezse) başa sarılır. */
  var video=document.getElementById("sn-video");
  if(video){
    var kaynak=video.querySelector("source");
    var adres=kaynak&&kaynak.getAttribute("src");
    if(azalt||!adres){K.dataset.video="poster";}
    else{
      var baslangic=parseFloat(getComputedStyle(K).getPropertyValue("--sn-v-bas"))||0;
      var oynat=function(){
        if(video.readyState<2)return;
        var p=video.play();
        if(p&&p.catch)p.catch(function(){});
      };
      video.addEventListener("loadedmetadata",function(){
        K.dataset.video="hazir";
        var sure=video.duration||8;
        if(baslangic>0&&baslangic<sure-0.4){
          try{video.currentTime=baslangic;}catch(e){}
        }
        oynat();
      });
      video.addEventListener("canplay",oynat);
      video.addEventListener("error",function(){K.dataset.video="yok";});
      video.addEventListener("ended",function(){
        try{video.currentTime=0;oynat();}catch(e){}
      });
      document.addEventListener("visibilitychange",function(){
        if(!document.hidden)oynat();
      });
      addEventListener("pointerdown",oynat,{passive:true,once:true});
      // Poster hemen görünür; video kısa süre sonra indirilip başlar.
      setTimeout(function(){video.preload="auto";video.load();},400);
    }
  }

  var rozet=document.getElementById("sn-durum");
  if(rozet&&PROGRAM){
    try{
      var parca=new Intl.DateTimeFormat("tr-TR",{timeZone:"Europe/Istanbul",weekday:"short",hour:"numeric",minute:"numeric",hour12:false}).formatToParts(new Date());
      var al=function(t){for(var i=0;i<parca.length;i++){if(parca[i].type===t)return parca[i].value;}return "";};
      var gunAd=al("weekday").slice(0,3);
      var gunler=["Paz","Pzt","Sal","Çar","Per","Cum","Cmt"];
      var gun=gunler.indexOf(gunAd);
      if(gun<0)gun=new Date().getDay();
      var dk=(parseInt(al("hour"),10)||0)*60+(parseInt(al("minute"),10)||0);
      var acik=false;
      (PROGRAM[gun]||[]).forEach(function(a){if(dk>=a[0]&&dk<a[1])acik=true;});
      rozet.hidden=false;
      rozet.textContent=acik?"Şu an açık":"Şu an kapalı";
      if(acik)rozet.classList.add("acik");
    }catch(e){}
  }

  if(!statik){
    var kartlar=[].slice.call(document.querySelectorAll("[data-egil]"));
    kartlar.forEach(function(kart){
      kart.addEventListener("pointermove",function(e){
        var r=kart.getBoundingClientRect();
        var x=(e.clientX-r.left)/r.width, y=(e.clientY-r.top)/r.height;
        kart.style.transform="perspective(900px) rotateY("+((x-.5)*9)+"deg) rotateX("+((.5-y)*9)+"deg) translateY(-8px)";
        kart.style.setProperty("--sn-mx",(x*100).toFixed(1)+"%");
        kart.style.setProperty("--sn-my",(y*100).toFixed(1)+"%");
      },{passive:true});
      kart.addEventListener("pointerleave",function(){kart.style.transform="";});
    });
  }

  /* Yükleme perdesi: sayaç önce ilerler, sayfa ve poster hazır olunca
     yüzde 100'e atlayıp yukarı doğru perde gibi açılır. */
  var perde=document.getElementById("sn-perde");
  if(perde){
    if(azalt){if(perde.parentNode)perde.parentNode.removeChild(perde);}
    else{
      var pyuzde=document.getElementById("sn-perde-yuzde"), pdeger=0, kapandi=false;
      var pkapat=function(){
        if(kapandi)return;
        kapandi=true;
        pdeger=100;
        if(pyuzde)pyuzde.textContent="100%";
        perde.style.setProperty("--sn-yuk","1");
        setTimeout(function(){
          perde.classList.add("bitti");
          setTimeout(function(){if(perde.parentNode)perde.parentNode.removeChild(perde);},900);
        },260);
      };
      var ptik=setInterval(function(){
        pdeger=Math.min(92,pdeger+Math.max(1.2,(92-pdeger)*.14));
        if(pyuzde)pyuzde.textContent=Math.round(pdeger)+"%";
        perde.style.setProperty("--sn-yuk",(pdeger/100).toFixed(3));
      },90);
      var phazir=function(){clearInterval(ptik);pkapat();};
      if(document.readyState==="complete"){setTimeout(phazir,340);}
      else{addEventListener("load",function(){setTimeout(phazir,240);},{once:true});}
      setTimeout(function(){if(document.getElementById("sn-perde"))phazir();},4200);
    }
  }

  /* Masaüstü özel imleç: nokta anında, halka yumuşak takip eder. */
  var imlec=document.querySelector(".sn-imlec"), halka=document.querySelector(".sn-imlec-halka");
  if(imlec&&halka&&!azalt&&matchMedia("(hover:hover) and (pointer:fine)").matches){
    K.dataset.imlec="1";
    var ix=innerWidth/2, iy=innerHeight/2, hx=ix, hy=iy, ilk=true;
    addEventListener("pointermove",function(e){
      ix=e.clientX;iy=e.clientY;
      if(ilk){hx=ix;hy=iy;ilk=false;}
      imlec.style.transform="translate3d("+ix+"px,"+iy+"px,0)";
      imlec.classList.add("gorunur");halka.classList.add("gorunur");
    },{passive:true});
    addEventListener("pointerdown",function(){imlec.classList.add("basili");},{passive:true});
    addEventListener("pointerup",function(){imlec.classList.remove("basili");},{passive:true});
    addEventListener("pointerleave",function(){imlec.classList.remove("gorunur");halka.classList.remove("gorunur");});
    (function halkaCiz(){
      hx+=(ix-hx)*.16;hy+=(iy-hy)*.16;
      halka.style.transform="translate3d("+hx.toFixed(1)+"px,"+hy.toFixed(1)+"px,0)";
      requestAnimationFrame(halkaCiz);
    })();
  }

  /* Manyetik butonlar: imleç yaklaştıkça düğme hafifçe çekilir. */
  if(!azalt&&matchMedia("(hover:hover) and (pointer:fine)").matches){
    [].slice.call(document.querySelectorAll(".sn-buton,.sn-ust-cta,.sn-bar-buton")).forEach(function(dugme){
      dugme.addEventListener("pointermove",function(e){
        var r=dugme.getBoundingClientRect();
        var x=(e.clientX-r.left-r.width/2)*.16, y=(e.clientY-r.top-r.height/2)*.26;
        dugme.style.transform="translate("+x.toFixed(1)+"px,"+y.toFixed(1)+"px)";
      },{passive:true});
      dugme.addEventListener("pointerleave",function(){dugme.style.transform="";});
    });
  }

  var tuval=document.querySelector(".sn-tuval");
  if(tuval&&!statik){
    var gl=null;
    try{gl=tuval.getContext("webgl",{antialias:false,alpha:false,depth:false,powerPreference:"low-power"})||tuval.getContext("experimental-webgl");}catch(e){gl=null;}
    if(!gl){K.dataset.tuval="yok";}
    else{
      var dpr=Math.min(window.devicePixelRatio||1,1.5)*.5;
      function boyut(){tuval.width=Math.max(1,Math.round(innerWidth*dpr));tuval.height=Math.max(1,Math.round(innerHeight*dpr));gl.viewport(0,0,tuval.width,tuval.height);}
      boyut();addEventListener("resize",boyut,{passive:true});
      var vs="attribute vec2 p;void main(){gl_Position=vec4(p,0.,1.);}";
      var fs="precision mediump float;uniform vec2 u_res;uniform vec2 u_m;uniform float u_t;uniform vec3 u_r;uniform vec3 u_r2;uniform float u_hiz;uniform float u_dalga;uniform float u_parca;void main(){vec2 q=(gl_FragCoord.xy-.5*u_res)/u_res.y;float tt=u_t*u_hiz;float d=length(q-vec2(u_m.x-.5,u_m.y-.5));float h=sin(q.x*2.1*u_dalga+tt*.22)+sin(q.y*2.7*u_dalga-tt*.17)+sin((q.x+q.y)*1.3*u_dalga+tt*.11);vec3 c=mix(vec3(.02,.024,.034),vec3(.06,.07,.095),.5+.26*h);c+=exp(-d*2.2)*.9*u_r;c+=exp(-length(q-vec2(.42,.32))*1.7)*.55*u_r2;float p=fract(sin(dot(floor(gl_FragCoord.xy/3.),vec2(12.9898,78.233))+tt*.4)*43758.5453);c+=step(1.-u_parca*.2,p)*u_r*.08;float g=fract(sin(dot(gl_FragCoord.xy,vec2(12.9898,78.233))+tt*.6)*43758.5453);c+=(g-.5)*.05;gl_FragColor=vec4(c,1.);}";
      function kur(tip,kaynak){var s=gl.createShader(tip);gl.shaderSource(s,kaynak);gl.compileShader(s);return s;}
      var p=gl.createProgram();
      gl.attachShader(p,kur(gl.VERTEX_SHADER,vs));
      gl.attachShader(p,kur(gl.FRAGMENT_SHADER,fs));
      gl.linkProgram(p);
      if(!gl.getProgramParameter(p,gl.LINK_STATUS)){K.dataset.tuval="yok";}
      else{
        gl.useProgram(p);
        var tampon=gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER,tampon);
        gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-1,-1,3,-1,-1,3]),gl.STATIC_DRAW);
        var konum=gl.getAttribLocation(p,"p");
        gl.enableVertexAttribArray(konum);
        gl.vertexAttribPointer(konum,2,gl.FLOAT,false,0,0);
        var uRes=gl.getUniformLocation(p,"u_res"),uT=gl.getUniformLocation(p,"u_t"),uM=gl.getUniformLocation(p,"u_m"),uR=gl.getUniformLocation(p,"u_r"),uR2=gl.getUniformLocation(p,"u_r2"),uH=gl.getUniformLocation(p,"u_hiz"),uD=gl.getUniformLocation(p,"u_dalga"),uP=gl.getUniformLocation(p,"u_parca");
        var renk=getComputedStyle(K).getPropertyValue("--sn-vurgu").trim()||"#c9a227";
        var m=renk.replace("#","");
        if(m.length===3)m=m[0]+m[0]+m[1]+m[1]+m[2]+m[2];
        var cr=parseInt(m.substr(0,2),16)/255||.79,cg=parseInt(m.substr(2,2),16)/255||.64,cb=parseInt(m.substr(4,2),16)/255||.15;
        gl.uniform3f(uR,cr*.75,cg*.55,cb*.35);
        /* Tohumlu arka plan: ikinci renk, hız, dalga ve partikül yoğunluğu
           işletmenin CSS değişkenlerinden okunur. */
        var stil=getComputedStyle(K);
        var oku=function(ad,v){var n=parseFloat(stil.getPropertyValue(ad));return isNaN(n)?v:n;};
        var renk2=stil.getPropertyValue("--sn-akis2").trim()||renk;
        var m2=renk2.replace("#","");
        if(m2.length===3)m2=m2[0]+m2[0]+m2[1]+m2[1]+m2[2]+m2[2];
        var r2r=parseInt(m2.substr(0,2),16)/255||.28,r2g=parseInt(m2.substr(2,2),16)/255||.36,r2b=parseInt(m2.substr(4,2),16)/255||.5;
        gl.uniform3f(uR2,r2r,r2g,r2b);
        gl.uniform1f(uH,oku("--sn-hiz",1));
        gl.uniform1f(uD,oku("--sn-dalga",1));
        gl.uniform1f(uP,oku("--sn-parca",.35));
        var mx=.5,my=.5;
        addEventListener("pointermove",function(e){mx=e.clientX/innerWidth;my=1-e.clientY/innerHeight;},{passive:true});
        var bas=performance.now(),kare=0,durdu=false;
        document.addEventListener("visibilitychange",function(){if(document.hidden)durdu=true;else if(kare<90){durdu=false;bas=performance.now();kare=0;requestAnimationFrame(ciz);}});
        function ciz(t){
          if(durdu)return;
          gl.uniform2f(uRes,tuval.width,tuval.height);
          gl.uniform2f(uM,mx,my);
          gl.uniform1f(uT,(t-bas)/1000);
          gl.drawArrays(gl.TRIANGLES,0,3);
          kare++;
          if(kare===90){
            var fps=90000/Math.max(1,performance.now()-bas);
            if(fps<30){K.dataset.tuval="zayif";durdu=true;return;}
          }
          requestAnimationFrame(ciz);
        }
        requestAnimationFrame(ciz);
      }
    }
  }

  var dalga=document.querySelector(".sn-dalga");
  if(dalga&&!statik){
    var c2=dalga.getContext("2d");
    if(c2){
      var t0=performance.now(),gorunur=true;
      if("IntersectionObserver" in window){new IntersectionObserver(function(g){gorunur=g[0].isIntersecting;},{threshold:.01}).observe(dalga);}
      (function dalgaCiz(t){
        if(!gorunur){requestAnimationFrame(dalgaCiz);return;}
        var w=dalga.width,h=dalga.height,s=(t-t0)/1000;
        c2.clearRect(0,0,w,h);
        for(var j=0;j<3;j++){
          c2.beginPath();
          c2.strokeStyle="rgba(201,162,39,"+(0.5-j*0.12)+")";
          c2.lineWidth=1.4;
          for(var x=0;x<=w;x+=8){
            var y=h/2+Math.sin(x/58+s*(0.8+j*0.3)+j)*38*Math.exp(-Math.pow((x-w/2)/w,2)*2.4);
            if(x===0)c2.moveTo(x,y);else c2.lineTo(x,y);
          }
          c2.stroke();
        }
        requestAnimationFrame(dalgaCiz);
      })(performance.now());
    }
  }
})();
"##;

#[cfg(test)]
mod acil_katman_testleri {
    use super::*;

    fn girdi(sektor: &str) -> Girdi {
        Girdi {
            ad: "Saruhanlı Oto Kurtarma".to_string(),
            adres: "Paşa Alanı, 10020 Balıkesir".to_string(),
            sektor: sektor.to_string(),
            tur: "servis",
            vurgu: "#14663d".to_string(),
            zemin: "#0d1b17".to_string(),
            yazi: "#f7f4ee".to_string(),
            foto: None,
            foto_atif: None,
            galeri: Vec::new(),
            galeri_ornek: true,
            galeri_not: None,
            galeri_atif: None,
            telefon: Some("905551112233".to_string()),
            telefon_goster: "0555 111 22 33".to_string(),
            konum: None,
            program: None,
            saat_metni: String::new(),
            saat_satirlari: Vec::new(),
            puan: None,
            yorum: None,
            karne: None,
            kaynak: "Google Maps".to_string(),
            gonderen: "Nexus".to_string(),
            anahtar: "test".to_string(),
            bolumler: Bolumler::default(),
            slogan: String::new(),
            cta: String::new(),
            kose: String::new(),
            tema: String::new(),
            ikincil: String::new(),
            metin: Metinler::default(),
            hikaye_foto: None,
            satici_tel: None,
            font_css: "",
            video: None,
            medya_tohum: 0,
            tasarim: None,
            profil: "klasik".to_string(),
            sinematik: SinematikAyar::default(),
            brif: None,
        }
    }

    fn sinematik_girdi(sahne: usize) -> Girdi {
        let mut g = girdi("Kuaför");
        g.profil = "sinematik".to_string();
        g.sinematik = SinematikAyar {
            hareket: "yuksek".to_string(),
            nesne: "halka".to_string(),
            sahne,
        };
        g.slogan = "Saç, sakal ve ritüel".to_string();
        g.cta = "Randevu al".to_string();
        g.galeri = vec![("image/webp".to_string(), "AAAA".to_string())];
        g.saat_satirlari = vec![("Pazartesi".to_string(), "10:00–20:00".to_string())];
        g.program = Some("[[600,1200],[],[],[],[],[],[]]".to_string());
        g
    }

    #[test]
    fn tohum_tutarli_ve_isleme_gore_farkli() {
        let a = tohum("Karizma Erkek Kuaförü", "Caferağa, Kadıköy");
        let b = tohum("Karizma Erkek Kuaförü", "Caferağa, Kadıköy");
        let c = tohum("Narin Ocak", "Caferağa, Kadıköy");
        assert_eq!(a, b, "aynı işletme aynı tohumu almalı");
        assert_ne!(a, c, "farklı işletme farklı tohum almalı");
        assert_eq!(a.len(), 6);
    }

    #[test]
    fn sinematik_hero_video_kesintisiz_dongude_oynar() {
        let mut g = sinematik_girdi(4);
        g.video = Some(VideoGirdi {
            dosya: "media/hero.mp4".to_string(),
            poster: "media/poster.webp".to_string(),
            atif: None,
        });
        let sayfa = uret(&g);
        for parca in [
            "autoplay",
            "muted",
            "loop",
            "playsinline",
            "preload=\"auto\"",
            "disablepictureinpicture",
            "sn-zemin-medya",
            "sn-zemin-perde",
            "class=\"sn-video bg-video\"",
            "hero-overlay",
            "backdrop-filter:blur(4px)",
            "data-zemin=\"medya\"",
            "position:fixed",
            "height:100vh",
            "object-fit:cover",
            "inset:0",
            "oynat()",
        ] {
            assert!(sayfa.contains(parca), "eksik: {parca}");
        }
        assert!(
            !sayfa.contains("hesapla=function"),
            "kaydırma ile scrub video döngüsünü kesmemeli"
        );
        assert!(
            !sayfa.contains("sn-hero-medya"),
            "medya hero kutusunda kalmamalı"
        );
    }

    #[test]
    fn sinematik_iskeleti_donusum_ve_yedekleri_tasir() {
        let sayfa = uret(&sinematik_girdi(6));
        for parca in [
            "data-profil=\"sinematik\"",
            "sn-halka",
            "sn-bar",
            "sn-durum",
            "application/ld+json",
            "prefers-reduced-motion",
            "html[data-js=\"1\"] .sn-sahne",
            "document.documentElement.dataset.js",
            "Yol tarifi",
            "https://wa.me/",
        ] {
            assert!(sayfa.contains(parca), "eksik: {parca}");
        }
        assert!(!sayfa.contains("<script src="), "harici betik olmamalı");
        assert!(!sayfa.contains("<link "), "harici stil/ön yükleme olmamalı");
        let stil_bas = sayfa.find("<style>").expect("stil bloğu");
        let stil_son = sayfa.find("</style>").expect("stil bloğu kapanışı");
        let stil = &sayfa[stil_bas..stil_son];
        assert!(
            stil.contains(".sn-sahne") && stil.contains("--sn-vurgu"),
            "sinematik CSS <style> bloğunun dışında kalmış"
        );
        assert_eq!(
            sayfa.matches("data-sahne=").count(),
            6,
            "6 sahne bekleniyordu"
        );
    }

    #[test]
    fn dunya_tohumdan_tutarli_ve_benzersiz() {
        let a = dunya_uret("a1b2c3", "#c9a227", "#08080a");
        let b = dunya_uret("a1b2c3", "#c9a227", "#08080a");
        let c = dunya_uret("0f0f0f", "#c9a227", "#08080a");
        assert_eq!(a.akis1, b.akis1, "aynı işletme aynı dünyayı almalı");
        assert_eq!(a.hiz, b.hiz);
        assert_ne!(a.akis1, c.akis1, "komşu işletme farklı renk almalı");
        assert!(a.hiz >= 0.55 && a.hiz <= 1.45);
        assert!(a.parca >= 0.16 && a.parca <= 0.78);
        assert_ne!(a.parca, c.parca, "partikül yoğunluğu değişmeli");
        assert_ne!(a.vinyet, c.vinyet, "vinyet değişmeli");
        assert!(a.akis1.starts_with('#') && a.akis1.len() == 7);
    }

    #[test]
    fn sinematik_dunya_sayfaya_gomulur_ve_isletmeye_gore_degisir() {
        let mut a = sinematik_girdi(6);
        a.ad = "Narin Ocak".to_string();
        a.adres = "Cihangir, Beyoğlu / İstanbul".to_string();
        // Aynı sektör, farklı işletme: dünya yine benzersiz olmalı.
        let mut b = a.clone();
        b.ad = "Karizma Erkek Kuaförü".to_string();
        b.adres = "Caferağa, Kadıköy / İstanbul".to_string();
        let sa = uret(&a);
        let sb = uret(&b);
        let al = |html: &str, anahtar: &str| {
            let bas = html.find(anahtar).expect("değişken") + anahtar.len();
            html[bas..].split(';').next().unwrap_or("").to_string()
        };
        for anahtar in ["--sn-akis1:", "--sn-akis2:", "--sn-hiz:", "--sn-parca:"] {
            assert_ne!(
                al(&sa, anahtar),
                al(&sb, anahtar),
                "aynı sektörde iki işletme farklı dünya almalı: {anahtar}"
            );
        }
        for parca in [
            "data-dunya=",
            "--sn-parca:",
            "--sn-hiz:",
            "u_parca",
            "snAkis",
        ] {
            assert!(sa.contains(parca), "eksik: {parca}");
        }
    }

    #[test]
    fn sinematik_premium_katman_taslari_basilir() {
        let sayfa = uret(&sinematik_girdi(6));
        for parca in [
            "sn-perde",
            "sn-perde-yuzde",
            "sn-vinyet",
            "sn-imlec-halka",
            "sn-serif",
            "sn-harf-kap",
            "snKay",
            "snHarf",
            "--sn-yuk",
            "sn-karart",
        ] {
            assert!(sayfa.contains(parca), "eksik premium parça: {parca}");
        }
        // Harf animasyonu no-JS'te metni gizlememeli: başlangıç yalnız data-js'te.
        assert!(
            sayfa.contains("html[data-js=\"1\"] .sn-h{"),
            "harf başlangıç durumu data-js'e bağlı olmalı"
        );
        // Azaltılmış harekette katmanlar kapanır.
        assert!(sayfa.contains("html[data-hareket=\"statik\"] .sn-perde{display:none}"));
        assert!(sayfa.contains("html[data-hareket=\"statik\"] .sn-serif-ic{animation:none}"));
        assert!(sayfa.contains("html[data-hareket=\"statik\"] .sn-imlec"));
        // Okunabilirlik karartması videonun içinde, video varken basılır.
        let mut video = sinematik_girdi(5);
        video.video = Some(VideoGirdi {
            dosya: "media/hero.mp4".to_string(),
            poster: "media/poster.webp".to_string(),
            atif: Some("Pexels · Beni Söz".to_string()),
        });
        let sayfa = uret(&video);
        assert!(sayfa.contains("sn-hero-video"), "video figürü");
        assert!(sayfa.contains("sn-karart"), "karartma katmanı");
        assert!(
            sayfa.contains("Görsel: Pexels · Beni Söz"),
            "atıf altbilgide"
        );
    }

    #[test]
    fn harf_harf_manset_ekran_okuyucuya_duz_metin_verir() {
        let satirlar = sn_satirlar("Karizma\nKuaför");
        assert_eq!(satirlar.matches("class=\"sn-satir\"").count(), 2);
        assert_eq!(satirlar.matches("class=\"sn-ekran\"").count(), 2);
        assert!(satirlar.contains("class=\"sn-h\""), "harf öğeleri");
        assert!(satirlar.matches("style=\"--i:").count() >= 12);
        assert!(satirlar.contains(">Karizma<"), "düz metin korunur");
    }

    #[test]
    fn sinematik_hero_bir_galeri_iki_kare_kullanir() {
        let mut g = sinematik_girdi(5);
        g.foto = Some(("image/webp".to_string(), "HEROKARE".to_string()));
        g.galeri = vec![
            ("image/webp".to_string(), "BIRKARE".to_string()),
            ("image/webp".to_string(), "IKIKARE".to_string()),
            ("image/webp".to_string(), "UCKARE".to_string()),
        ];
        let sayfa = uret(&g);
        assert!(sayfa.contains("HEROKARE"), "hero fotoğrafı basılmalı");
        assert_eq!(
            sayfa.matches("class=\"sn-kare\"").count(),
            2,
            "galeride en fazla 2 kare"
        );
        assert!(sayfa.contains("IKIKARE"));
        // Kartlar da kareleri kullanır; üçüncü kare galeriye girmez.
        let galeri = sayfa
            .split("<div class=\"sn-galeri\">")
            .nth(1)
            .and_then(|kalan| kalan.split("</div>").next())
            .expect("galeri bölümü");
        assert!(!galeri.contains("UCKARE"), "üçüncü kare galeriye girmemeli");
        assert_eq!(
            sayfa.matches("class=\"sn-kart-foto sn-kf").count(),
            4,
            "dört hizmet kartı fotoğrafla açılır"
        );
        assert_eq!(
            sayfa.matches("<img").count(),
            3,
            "1 hero + 2 galeri; kartlar CSS kuralını paylaşır"
        );
        assert!(
            sayfa.matches("background-image:url(data:image").count() <= 4,
            "kart fotoğrafları tek kez gömülür"
        );
        // Fotoğraf hero'su ve kartlar Ken Burns ile akar; soyut tuval gizlenir.
        assert!(sayfa.contains("sn-ken"), "hero Ken Burns sınıfı");
        assert!(
            sayfa.contains("@keyframes snKenBurns"),
            "Ken Burns animasyonu"
        );
        assert!(sayfa.contains("--kb-s:"), "tohumlu Ken Burns süresi");
        assert!(
            sayfa.contains("data-medya=\"1\""),
            "medya varken tuval kapalı"
        );
    }

    #[test]
    fn sinematik_sahne_sayisi_3_6_araligina_kirpilir() {
        assert_eq!(uret(&sinematik_girdi(1)).matches("data-sahne=").count(), 3);
        assert_eq!(uret(&sinematik_girdi(9)).matches("data-sahne=").count(), 6);
        assert_eq!(uret(&sinematik_girdi(4)).matches("data-sahne=").count(), 4);
    }

    #[test]
    fn sinematik_butce_fotograflari_dusurur() {
        let mut g = sinematik_girdi(4);
        g.galeri = vec![
            ("image/webp".to_string(), "A".repeat(400_000)),
            ("image/webp".to_string(), "B".repeat(400_000)),
        ];
        let (html, kirpilan) = uret_butceli(&g, 320_000);
        assert!(kirpilan >= 1, "bütçe için en az bir kare düşmeliydi");
        assert!(html.len() <= 320_000, "bütçe aşıldı: {}", html.len());
    }

    #[test]
    fn sinematik_olmayan_profil_butceden_etkilenmez() {
        let mut g = sinematik_girdi(4);
        g.profil = "ajans".to_string();
        let once = kucult(&uret(&g)).len();
        let (sonra, kirpilan) = uret_butceli(&g, 120_000);
        assert_eq!(kirpilan, 0, "ajans profilinde kare atılmamalı");
        assert_eq!(sonra.len(), once);
    }

    #[test]
    fn cekici_sayfasinda_acil_katman_cizilir() {
        let sayfa = uret(&girdi("Çekici"));
        for parca in [
            "class=\"hero-rozetler\"",
            "Canlı konum gönder",
            "id=\"asistan-dugme\"",
            "data-durum=\"Arıza\"",
            "id=\"yorum-kaydirici\"",
            "kenburns",
            "class=\"mobil-bar\"",
            "Merhaba, yolda kaldım. Konumum: ",
        ] {
            assert!(sayfa.contains(parca), "eksik parça: {parca}");
        }
    }

    #[test]
    fn sektorel_tasarim_kimligi_sayfaya_yazilir() {
        let mut g = girdi("Çekici");
        g.tasarim = Some(TasarimGirdi {
            baslik: "'Space Grotesk',sans-serif".to_string(),
            govde: "'Geist',sans-serif".to_string(),
            aciliyet: 3,
            sinematik: true,
            rozetler: vec![
                "7/24 aktif ekip".to_string(),
                "Ortalama varış: 15–20 dk".to_string(),
            ],
            cta: "Hemen ara".to_string(),
            vurgu2: "#ffd166".to_string(),
            ad: "Çekici & yol yardım".to_string(),
            grup: "acil".to_string(),
            hareket: "canli".to_string(),
        });
        let sayfa = uret(&g);
        assert!(sayfa.contains("data-hareket=\"canli\""));
        assert!(sayfa.contains("sinema-kare"));
        assert!(sayfa.contains("7/24 aktif ekip"));
        assert!(sayfa.contains("Hemen ara"));
        assert!(sayfa.contains("--accent-2:#ffd166"));
    }

    #[test]
    fn diger_sektorlerde_asistan_cizilmez() {
        let sayfa = uret(&girdi("Kafe"));
        assert!(!sayfa.contains("id=\"asistan-dugme\""));
        assert!(!sayfa.contains("class=\"mobil-bar\""));
        assert!(!sayfa.contains("class=\"hero-rozetler\""));
        assert!(sayfa.contains("kenburns"));
    }

    #[test]
    fn ajans_profili_alti_bolum_ve_sinematik_katman_uretir() {
        let mut g = girdi("Kafe");
        g.profil = "ajans".to_string();
        let sayfa = uret(&g);
        for parca in [
            "data-profil=\"ajans\"",
            "class=\"aj-hero\"",
            "id=\"hakkimizda\"",
            "id=\"hizmetler\"",
            "id=\"ai-asistan\"",
            "id=\"guven\"",
            "id=\"isler\"",
            "id=\"iletisim\"",
            "id=\"aj-glow\"",
            "data-stil=\"luxe-sinema\"",
            "data-aj-sayac",
            "aj-sss-madde",
            "aj-cam",
            "backdrop-filter",
            "kenburns",
            "IntersectionObserver",
            "data-aj-reveal",
            "id=\"mobil-dugme\"",
            "class=\"serit\">Taslak demo ·",
        ] {
            assert!(sayfa.contains(parca), "eksik parça: {parca}");
        }
        assert_eq!(sayfa.matches("data-aj-section").count(), 6);
        assert!(!sayfa.contains("<script src="), "harici script olmamalı");
        assert!(!sayfa.contains("cdn."), "CDN olmamalı");
    }

    #[test]
    fn ajans_acil_profilinde_asistan_ve_mobil_bar_korunur() {
        let mut g = girdi("Çekici");
        g.profil = "ajans".to_string();
        let sayfa = uret(&g);
        for parca in [
            "class=\"mobil-bar\"",
            "Canlı Konum Gönder",
            "id=\"asistan-dugme\"",
            "hero-rozetler",
        ] {
            assert!(sayfa.contains(parca), "eksik acil katman: {parca}");
        }
    }

    /// Elle ölçüm: `cargo test --all-targets kucultme_raporu -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn kucultme_raporu() {
        let mut g = girdi("Çekici");
        g.profil = "ajans".to_string();
        let ham = uret(&g);
        let kucuk = kucult(&ham);
        println!(
            "ham: {} bayt · küçük: {} bayt · kazanç: {:.2}%",
            ham.len(),
            kucuk.len(),
            100.0 * (ham.len() - kucuk.len()) as f64 / ham.len() as f64
        );
    }

    #[test]
    fn uretilen_tam_sayfa_kucultulunce_kuculur() {
        let mut g = girdi("Çekici");
        g.profil = "ajans".to_string();
        let ham = uret(&g);
        let kucuk = kucult(&ham);
        assert!(kucuk.len() < ham.len(), "tam sayfa küçülmeli");
        assert!(kucuk.contains("guven"));
        assert!(kucuk.contains("data-aj-sayac"));
        assert!(kucuk.contains("</body>"), "kapanış etiketi korunmalı");
    }

    #[test]
    fn kucultme_boyutu_dusurur_ve_yapiyi_korur() {
        let ham = "<!DOCTYPE html>\n<html lang=\"tr\">\n  <head>\n    <title>Deneme</title>\n    <style>\n      .a { color: red; }\n    </style>\n  </head>\n  <body>\n    <p class=\"b\">Merhaba dünya</p>\n    <script>\n      var a = 1; // yorum\n    </script>\n  </body>\n</html>\n";
        let kucuk = kucult(ham);
        assert!(kucuk.len() < ham.len(), "küçültme boyutu düşürmeli");
        assert!(kucuk.contains("Merhaba dünya"));
        assert!(kucuk.contains("</body>"), "kapanış etiketi korunmalı");
        assert!(!kucuk.contains("// yorum"), "JS yorumu atılmalı");
    }

    #[test]
    fn guven_bolumu_ve_sektor_stili_uretir() {
        let mut g = girdi("Restoran");
        g.profil = "ajans".to_string();
        g.tasarim = Some(TasarimGirdi {
            baslik: "'Instrument Serif',serif".to_string(),
            govde: "'Sora',sans-serif".to_string(),
            aciliyet: 1,
            sinematik: true,
            rozetler: Vec::new(),
            cta: "Rezervasyon".to_string(),
            vurgu2: "#d9b98a".to_string(),
            ad: "Mutfak".to_string(),
            grup: "mutfak".to_string(),
            hareket: "sakin".to_string(),
        });
        let sayfa = uret(&g);
        for parca in [
            "id=\"guven\"",
            "data-stil=\"luxe-sinema\"",
            "data-aj-sayac=\"4.9\"",
            "aj-kademe aj-cam aj-kademe-one",
            "class=\"aj-sss-madde\"",
            "7/24 Kesintisiz Hizmet",
            "aj-guven-rozet",
            "aj-hizmet-ikon",
            "class=\"aj-ikon\"",
        ] {
            assert!(sayfa.contains(parca), "eksik güven parçası: {parca}");
        }
        assert_eq!(sayfa.matches("data-aj-section").count(), 6);
    }

    #[test]
    fn ai_brifi_ajans_ciktisini_gercekten_yonlendirir() {
        let mut g = girdi("Kafe");
        g.profil = "ajans".to_string();
        g.brif = Some(crate::tasarim_brifi::Brif {
            profil: "ajans".to_string(),
            palet: crate::tasarim_brifi::Palet {
                vurgu: "#ff0066".to_string(),
                vurgu2: "#00e5ff".to_string(),
                zemin: "#0b0b10".to_string(),
                yazi: "#f5f5f7".to_string(),
            },
            hareket: "sakin".to_string(),
            efektler: crate::tasarim_brifi::Efektler {
                kenburns: false,
                glass: false,
                ..Default::default()
            },
            bolumler: vec![
                crate::tasarim_brifi::Bolum {
                    baslik: "AI başlığı|ikinci satır".to_string(),
                    metin: "AI alt metni".to_string(),
                    ..Default::default()
                },
                crate::tasarim_brifi::Bolum {
                    baslik: "AI manifestosu".to_string(),
                    metin: "Ajanın verdiği manifesto metni.".to_string(),
                    ..Default::default()
                },
            ],
            cta: "Brif CTA".to_string(),
            ..Default::default()
        });
        let sayfa = uret(&g);
        for parca in [
            "--accent:#ff0066",
            "--accent-2:#00e5ff",
            "data-hareket=\"sakin\"",
            "data-kenburns=\"0\"",
            "data-glass=\"0\"",
            "başlığı",
            "manifestosu",
            "Brif CTA",
        ] {
            assert!(sayfa.contains(parca), "brif yansımadı: {parca}");
        }
    }
}
