"""SonucOnbellek altyapisini (demo kaynagi ve sektor verimi icin) geri koyar."""

from pathlib import Path

YOL = Path("src/main.rs")
metin = YOL.read_text(encoding="utf-8")

ANCA = 'const KONUM_TTL_GUN: i64 = 180;\n'
if ANCA not in metin:
    raise SystemExit("anca bulunamadi")

EK = '''
const SONUC_ONBELLEK_DOSYASI: &str = "sonuc_onbellek.json";

/// Sonuç önbelleğinin varsayılan geçerlilik süresi (gün). `SONUC_TTL_GUN=0`
/// önbelleği tamamen kapatır, böylece her tarama canlı veri getirir.
const SONUC_TTL_GUN: i64 = 7;

fn sonuc_ttl_gun() -> i64 {
    std::env::var("SONUC_TTL_GUN")
        .ok()
        .and_then(|deger| deger.trim().parse::<i64>().ok())
        .unwrap_or(SONUC_TTL_GUN)
        .max(0)
}

/// Bir ilçe + sektör için saklanan arama sonucu.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SonucKaydi {
    tarih: String,
    siteli: usize,
    leads: Vec<BusinessLead>,
}

/// Önbellekten dönen kayıt. `gun` = kaç gün önce çekildiği.
struct SonucOnbellekKaydi {
    leads: Vec<BusinessLead>,
    siteli: usize,
    gun: i64,
}

/// İlçe + sektör başına tarama sonuçları. Aynı ilçe kısa süre içinde tekrar
/// tarandığında Google kotası ikinci kez harcanmaz.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct SonucOnbellek {
    #[serde(default)]
    kayitlar: HashMap<String, SonucKaydi>,
}

impl SonucOnbellek {
    fn yukle() -> Self {
        fs::read_to_string(SONUC_ONBELLEK_DOSYASI)
            .ok()
            .and_then(|metin| serde_json::from_str(&metin).ok())
            .unwrap_or_default()
    }

    fn kaydet(&self) {
        if let Ok(metin) = serde_json::to_string(self) {
            let _ = fs::write(SONUC_ONBELLEK_DOSYASI, metin);
        }
    }

    fn anahtar(il: &str, ilce: &str, sektor: &str) -> String {
        format!(
            "{}|{}|{}",
            arama_anahtari(il),
            arama_anahtari(ilce),
            arama_anahtari(sektor)
        )
    }

    /// TTL içindeki kayıt; yoksa veya eskimişse `None`.
    fn al(&self, il: &str, ilce: &str, sektor: &str) -> Option<SonucOnbellekKaydi> {
        let ttl = sonuc_ttl_gun();
        if ttl == 0 {
            return None;
        }
        let kayit = self.kayitlar.get(&Self::anahtar(il, ilce, sektor))?;
        let gun = gun_farki(&kayit.tarih, &bugun_anahtari())?;
        if gun < 0 || gun > ttl {
            return None;
        }
        Some(SonucOnbellekKaydi {
            leads: kayit.leads.clone(),
            siteli: kayit.siteli,
            gun,
        })
    }

    fn ekle(&mut self, il: &str, ilce: &str, sektor: &str, leads: &[BusinessLead], siteli: usize) {
        self.kayitlar.insert(
            Self::anahtar(il, ilce, sektor),
            SonucKaydi {
                tarih: bugun_anahtari(),
                siteli,
                leads: leads.to_vec(),
            },
        );
    }
}
'''

YOL.write_text(metin.replace(ANCA, ANCA + EK, 1), encoding="utf-8")
print("onbellek geri eklendi")
