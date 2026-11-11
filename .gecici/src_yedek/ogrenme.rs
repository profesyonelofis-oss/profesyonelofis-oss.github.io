//! Hafif öğrenme hafızası.
//!
//! `memory/agent_learning.json` yalnız metinsel kayıt tutar: sektör, seçilen
//! aile, animasyon sınıfı, onaylanan palet, hareket profili ve serbest metin
//! revizyon notu. Dosya birkaç kilobayt kalır (en fazla 400 kayıt); demo
//! üretimi bu hafızayı sorgulayarak daha önce onaylanmış paleti ve başarılı
//! animasyon sınıfını otomatik uygular.
//!
//! Yazma atomiktir (geçici dosya + rename) ve hatalar sessizce yutulur:
//! hafıza bozulsa bile demo üretimi durmaz.

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Hafıza dosyası (çalışma dizinine göre).
pub const HAFIZA_YOLU: &str = "memory/agent_learning.json";
/// Dosyanın şişmemesi için üst sınır: en eski kayıtlar düşer.
const AZAMI_KAYIT: usize = 400;
/// Animasyon alanına yazılabilen sınıflar. Eski "onayli-palet" etiketi
/// animasyon değil onay işaretidir; ders sorgusunda animasyon sayılmaz.
const ANIMASYONLAR: [&str; 2] = ["kenburns+sinema", "kenburns"];

/// Bir demoda kullanılan üç renk.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Palet {
    pub vurgu: String,
    pub zemin: String,
    pub yazi: String,
}

impl Palet {
    fn dolu(&self) -> bool {
        !self.vurgu.trim().is_empty()
            && !self.zemin.trim().is_empty()
            && !self.yazi.trim().is_empty()
    }
}

/// Tek öğrenme kaydı: aynı sektör + animasyon + palet + hareket tekrarında
/// `sayi` artar, yeni satır açılmaz. Böylece dosya küçük kalır.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kayit {
    pub sektor: String,
    pub aile: String,
    pub animasyon: String,
    pub palet: Palet,
    pub hareket: String,
    /// "uret": otomatik üretim · "onay": kullanıcı revizyonu onayladı.
    pub kaynak: String,
    /// Serbest metin revizyon notu (en fazla 180 karakter).
    pub not: String,
    pub sayi: u32,
    pub son: String,
}

/// Dosyanın tamamı.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hafiza {
    pub surum: u32,
    pub kayitlar: Vec<Kayit>,
}

impl Default for Hafiza {
    fn default() -> Self {
        Self {
            surum: 1,
            kayitlar: Vec::new(),
        }
    }
}

fn bugun() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn yukle_yol(yol: &Path) -> Hafiza {
    fs::read_to_string(yol)
        .ok()
        .and_then(|metin| serde_json::from_str(&metin).ok())
        .unwrap_or_default()
}

fn yaz_yol(yol: &Path, hafiza: &Hafiza) {
    if let Some(klasor) = yol.parent() {
        let _ = fs::create_dir_all(klasor);
    }
    let Ok(metin) = serde_json::to_string_pretty(hafiza) else {
        return;
    };
    let gecici = yol.with_extension("json.tmp");
    if fs::write(&gecici, metin).is_ok() {
        let _ = fs::rename(&gecici, yol);
    }
}

/// Yeni kaydı kurar; metin alanları tek yerde kırpılır.
fn kayit(
    sektor: &str,
    aile: &str,
    animasyon: &str,
    palet: Palet,
    hareket: &str,
    kaynak: &str,
    not: &str,
) -> Kayit {
    Kayit {
        sektor: sektor.trim().to_string(),
        aile: aile.to_string(),
        animasyon: animasyon.to_string(),
        palet,
        hareket: hareket.to_string(),
        kaynak: kaynak.to_string(),
        not: not.trim().chars().take(180).collect(),
        sayi: 1,
        son: bugun(),
    }
}

fn eslesiyor(kayit: &Kayit, yeni: &Kayit) -> bool {
    kayit.sektor.eq_ignore_ascii_case(&yeni.sektor)
        && kayit.animasyon == yeni.animasyon
        && kayit.palet == yeni.palet
        && kayit.hareket == yeni.hareket
}

/// Kaydı ekler veya aynı dörtlüyü sayar; ardından sınırı uygular.
/// Güncellenen kayıt listenin sonuna taşınır: aynı tarihli kayıtlarda
/// "en yeni onay" gerçekten en son onaylanan olur.
fn ekle(hafiza: &mut Hafiza, yeni: Kayit) {
    if yeni.sektor.is_empty() || !yeni.palet.dolu() {
        return;
    }
    if let Some(pozisyon) = hafiza.kayitlar.iter().position(|k| eslesiyor(k, &yeni)) {
        let mut kayit = hafiza.kayitlar.remove(pozisyon);
        kayit.sayi = kayit.sayi.saturating_add(1);
        kayit.son = yeni.son;
        if yeni.kaynak == "onay" {
            kayit.kaynak = "onay".to_string();
            // Eski "onayli-palet" işaretli kayıt gerçek sınıfa yükselir.
            kayit.animasyon = yeni.animasyon;
        }
        if !yeni.not.is_empty() {
            kayit.not = yeni.not;
        }
        hafiza.kayitlar.push(kayit);
    } else {
        hafiza.kayitlar.push(yeni);
    }
    if hafiza.kayitlar.len() > AZAMI_KAYIT {
        let fazla = hafiza.kayitlar.len() - AZAMI_KAYIT;
        hafiza.kayitlar.drain(0..fazla);
    }
}

fn ekle_yol(yol: &Path, yeni: Kayit) {
    let mut hafiza = yukle_yol(yol);
    ekle(&mut hafiza, yeni);
    yaz_yol(yol, &hafiza);
}

/// Demo üretildikten sonra çağrılır: kullanılan kimliği hafızaya işler.
pub fn uretim(
    sektor: &str,
    aile: &str,
    animasyon: &str,
    vurgu: &str,
    zemin: &str,
    yazi: &str,
    hareket: &str,
) {
    ekle_yol(
        Path::new(HAFIZA_YOLU),
        kayit(
            sektor,
            aile,
            animasyon,
            Palet {
                vurgu: vurgu.to_string(),
                zemin: zemin.to_string(),
                yazi: yazi.to_string(),
            },
            hareket,
            "uret",
            "",
        ),
    );
}

/// Kullanıcı revizyonu onaylandığında çağrılır: palet, animasyon sınıfı ve
/// serbest metin not bir sonraki üretimde varsayılan olur. `not` boş
/// bırakılırsa mevcut not korunur.
pub fn onay(
    sektor: &str,
    aile: &str,
    animasyon: &str,
    vurgu: &str,
    zemin: &str,
    yazi: &str,
    not: &str,
) {
    onay_yol(
        Path::new(HAFIZA_YOLU),
        kayit(
            sektor,
            aile,
            animasyon,
            Palet {
                vurgu: vurgu.to_string(),
                zemin: zemin.to_string(),
                yazi: yazi.to_string(),
            },
            "onay",
            "onay",
            not,
        ),
    );
}

/// Onay kaydı yazılırken eski "onayli-palet" işaretli kayıtlar o sektörden
/// temizlenir; gerçek onay geçmişi korunur, en yeni onay dersi belirler.
fn onay_yol(yol: &Path, yeni: Kayit) {
    let mut hafiza = yukle_yol(yol);
    hafiza.kayitlar.retain(|k| {
        !(k.sektor.eq_ignore_ascii_case(&yeni.sektor) && k.animasyon == "onayli-palet")
    });
    ekle(&mut hafiza, yeni);
    yaz_yol(yol, &hafiza);
}

/// Hafızadaki kayıtlar, en yeni önce. Arayüz listesi bu sırayı kullanır;
/// `sil` da aynı sıradaki indeksi kabul eder.
pub fn kayitlar() -> Vec<Kayit> {
    kayitlar_yol(Path::new(HAFIZA_YOLU))
}

fn kayitlar_yol(yol: &Path) -> Vec<Kayit> {
    let mut kayitlar = yukle_yol(yol).kayitlar;
    kayitlar.reverse();
    kayitlar
}

/// Listede `indeks`. kaydı siler (0 = en yeni). Kayıt yoksa `false`.
pub fn sil(indeks: usize) -> bool {
    sil_yol(Path::new(HAFIZA_YOLU), indeks)
}

fn sil_yol(yol: &Path, indeks: usize) -> bool {
    let mut hafiza = yukle_yol(yol);
    let adet = hafiza.kayitlar.len();
    if indeks >= adet {
        return false;
    }
    // `kayitlar()` ters çevirir; dosyadaki karşılığı sondan sayılır.
    hafiza.kayitlar.remove(adet - 1 - indeks);
    yaz_yol(yol, &hafiza);
    true
}

/// Tüm kayıtları siler; dosya boş şema olarak kalır.
pub fn temizle() {
    temizle_yol(Path::new(HAFIZA_YOLU));
}

fn temizle_yol(yol: &Path) {
    let mut hafiza = yukle_yol(yol);
    hafiza.kayitlar.clear();
    yaz_yol(yol, &hafiza);
}

/// Bir sonraki üretimde kullanılacak ders.
#[derive(Clone, Debug, Default)]
pub struct Ders {
    /// Yalnızca kullanıcının açıkça onayladığı palet. Otomatik (`uret`)
    /// kayıtlar buraya gelmez: sektörün kendi paleti her zaman önceliklidir.
    pub palet: Option<Palet>,
    /// Başarılı bulunan animasyon sınıfı (varsa).
    pub animasyon: Option<String>,
    /// Son revizyon notları (en yeni önce, en fazla 3).
    pub notlar: Vec<String>,
    /// Kaç kayıt incelendi (arayüzde gösterilir).
    pub kayit_sayisi: usize,
}

/// Sektörün hafızadaki dersi. Renk yalnızca kullanıcı onayıyla uygulanır;
/// birden çok onay varsa en yenisi kazanır. Onay yoksa `palet` boş kalır ve
/// üretim sektörün kendi rejim paletini kullanır. Animasyon alanı onaylı ya
/// da en çok kullanılan kayıttan gelir ve yalnız bilinen sınıflar için döner.
pub fn ders(sektor: &str) -> Ders {
    ders_yol(sektor, Path::new(HAFIZA_YOLU))
}

fn ders_yol(sektor: &str, yol: &Path) -> Ders {
    let hafiza = yukle_yol(yol);
    let sektor = sektor.trim();
    let ilgili: Vec<&Kayit> = hafiza
        .kayitlar
        .iter()
        .filter(|kayit| kayit.sektor.eq_ignore_ascii_case(sektor) && kayit.palet.dolu())
        .collect();
    if ilgili.is_empty() {
        return Ders {
            kayit_sayisi: hafiza.kayitlar.len(),
            ..Ders::default()
        };
    }

    // Onaylılar tarihe göre sıralanır; aynı tarihte dosya sırası korunur ve
    // son eleman (en son onaylanan) kazanır.
    let mut onayli: Vec<&Kayit> = ilgili
        .iter()
        .copied()
        .filter(|kayit| kayit.kaynak == "onay" || kayit.animasyon == "onayli-palet")
        .collect();
    onayli.sort_by(|a, b| a.son.cmp(&b.son));
    // Renk sözleşmesi: yalnızca en yeni onay paleti uygular. Otomatik üretim
    // kayıtları sektör paletini ezemez; animasyon seçimi öğrenmeye devam eder.
    let onayli_palet = onayli.pop();
    let secili = onayli_palet.or_else(|| ilgili.iter().copied().max_by_key(|kayit| kayit.sayi));

    let notlar: Vec<String> = ilgili
        .iter()
        .rev()
        .filter(|kayit| !kayit.not.trim().is_empty())
        .take(3)
        .map(|kayit| kayit.not.clone())
        .collect();
    let animasyon = secili.and_then(|kayit| {
        ANIMASYONLAR
            .contains(&kayit.animasyon.as_str())
            .then(|| kayit.animasyon.clone())
    });

    Ders {
        palet: onayli_palet.map(|kayit| kayit.palet.clone()),
        animasyon,
        notlar,
        kayit_sayisi: hafiza.kayitlar.len(),
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gecici(ad: &str) -> std::path::PathBuf {
        let mut yol = std::env::temp_dir();
        yol.push(format!("nexus_hafiza_{ad}_{}.json", std::process::id()));
        let _ = fs::remove_file(&yol);
        yol
    }

    fn palet(vurgu: &str) -> Palet {
        Palet {
            vurgu: vurgu.to_string(),
            zemin: "#080d13".to_string(),
            yazi: "#f6f2ea".to_string(),
        }
    }

    #[test]
    fn ayni_uclu_yeni_satir_acmaz_sayaci_artirir() {
        let yol = gecici("dedupe");
        let mut hafiza = Hafiza::default();
        for _ in 0..2 {
            ekle(
                &mut hafiza,
                kayit(
                    "Çekici",
                    "acil",
                    "kenburns+sinema",
                    palet("#ff7a1a"),
                    "canli",
                    "uret",
                    "",
                ),
            );
        }
        yaz_yol(&yol, &hafiza);
        let okunan = yukle_yol(&yol);
        assert_eq!(okunan.kayitlar.len(), 1);
        assert_eq!(okunan.kayitlar[0].sayi, 2);
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn sinir_asilinca_en_eski_kayit_duser() {
        let mut hafiza = Hafiza::default();
        for sira in 0..(AZAMI_KAYIT + 5) {
            ekle(
                &mut hafiza,
                kayit(
                    &format!("Sektör {sira}"),
                    "genel",
                    "kenburns",
                    palet("#123456"),
                    "dengeli",
                    "uret",
                    "",
                ),
            );
        }
        assert_eq!(hafiza.kayitlar.len(), AZAMI_KAYIT);
    }

    #[test]
    fn onay_serbest_notu_ve_animasyonu_saklar() {
        let yol = gecici("onay");
        onay_yol(
            &yol,
            kayit(
                "Klinik",
                "saglik",
                "kenburns+sinema",
                palet("#1b8fa8"),
                "onay",
                "onay",
                "zemin daha açık olsun, yazılar biraz daha büyüsün",
            ),
        );
        let ders = ders_yol("Klinik", &yol);
        assert_eq!(ders.palet.expect("palet").vurgu, "#1b8fa8");
        assert_eq!(ders.animasyon.as_deref(), Some("kenburns+sinema"));
        assert_eq!(
            ders.notlar.first().map(String::as_str),
            Some("zemin daha açık olsun, yazılar biraz daha büyüsün")
        );
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn otomatik_kayit_sektor_paletini_ezmez() {
        let yol = gecici("otomatik_palet");
        let mut hafiza = Hafiza::default();
        for _ in 0..5 {
            ekle(
                &mut hafiza,
                kayit(
                    "Çekici",
                    "acil",
                    "kenburns+sinema",
                    palet("#b4551f"),
                    "canli",
                    "uret",
                    "",
                ),
            );
        }
        yaz_yol(&yol, &hafiza);
        let ders = ders_yol("Çekici", &yol);
        assert!(ders.palet.is_none(), "otomatik kayıt rengi ezmemeli");
        assert_eq!(ders.animasyon.as_deref(), Some("kenburns+sinema"));
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn onay_renk_paletini_gecer() {
        let yol = gecici("onay_palet");
        let mut hafiza = Hafiza::default();
        ekle(
            &mut hafiza,
            kayit(
                "Çekici",
                "acil",
                "kenburns",
                palet("#ff7a1a"),
                "canli",
                "uret",
                "",
            ),
        );
        ekle(
            &mut hafiza,
            kayit(
                "Çekici",
                "acil",
                "kenburns",
                palet("#112233"),
                "canli",
                "onay",
                "marka rengi",
            ),
        );
        yaz_yol(&yol, &hafiza);
        let ders = ders_yol("Çekici", &yol);
        assert_eq!(ders.palet.expect("onaylı palet").vurgu, "#112233");
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn eski_onay_etiketi_animasyon_sayilmaz() {
        let yol = gecici("eski");
        let mut hafiza = Hafiza::default();
        ekle(
            &mut hafiza,
            kayit(
                "Klinik",
                "saglik",
                "onayli-palet",
                palet("#123456"),
                "onay",
                "onay",
                "eski kayıt",
            ),
        );
        yaz_yol(&yol, &hafiza);
        let ders = ders_yol("Klinik", &yol);
        assert!(ders.palet.is_some());
        assert!(ders.animasyon.is_none());
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn en_yeni_onay_dersi_belirler() {
        let yol = gecici("yeni_onay");
        let mut hafiza = Hafiza::default();
        ekle(
            &mut hafiza,
            kayit(
                "Klinik",
                "saglik",
                "kenburns",
                palet("#111111"),
                "onay",
                "onay",
                "ilk onay",
            ),
        );
        ekle(
            &mut hafiza,
            kayit(
                "Klinik",
                "saglik",
                "kenburns+sinema",
                palet("#1b8fa8"),
                "onay",
                "onay",
                "ikinci onay",
            ),
        );
        yaz_yol(&yol, &hafiza);
        let ders = ders_yol("Klinik", &yol);
        assert_eq!(ders.palet.expect("palet").vurgu, "#1b8fa8");
        assert_eq!(ders.animasyon.as_deref(), Some("kenburns+sinema"));
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn sil_ve_temizle_dosyayi_kucultur() {
        let yol = gecici("sil");
        let mut hafiza = Hafiza::default();
        ekle(
            &mut hafiza,
            kayit(
                "Restoran",
                "mutfak",
                "kenburns+sinema",
                palet("#b4551f"),
                "dengeli",
                "uret",
                "",
            ),
        );
        ekle(
            &mut hafiza,
            kayit(
                "Klinik",
                "saglik",
                "kenburns",
                palet("#1b8fa8"),
                "sakin",
                "uret",
                "",
            ),
        );
        yaz_yol(&yol, &hafiza);
        let liste = kayitlar_yol(&yol);
        assert_eq!(liste.len(), 2);
        assert_eq!(liste[0].sektor, "Klinik");
        assert!(sil_yol(&yol, 0));
        let kalan = kayitlar_yol(&yol);
        assert_eq!(kalan.len(), 1);
        assert_eq!(kalan[0].sektor, "Restoran");
        assert!(!sil_yol(&yol, 5));
        temizle_yol(&yol);
        assert!(kayitlar_yol(&yol).is_empty());
        let _ = fs::remove_file(&yol);
    }

    #[test]
    fn bos_sektor_kaydedilmez() {
        let mut hafiza = Hafiza::default();
        ekle(
            &mut hafiza,
            kayit(
                "  ",
                "genel",
                "kenburns",
                palet("#111111"),
                "dengeli",
                "uret",
                "",
            ),
        );
        assert!(hafiza.kayitlar.is_empty());
    }
}
