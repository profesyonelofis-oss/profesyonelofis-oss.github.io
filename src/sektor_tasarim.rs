//! Sektörel tasarım hafızası.
//!
//! Her sektör için dönüşüm odaklı bir görsel kimlik tutar: palet, tipografi
//! çifti, aciliyet seviyesi, hero rozetleri ve birincil çağrı metni. Demo
//! üreticisi bu kimliği otomatik seçer; panelden renk yazılmadıysa sektörün
//! kendi paleti kullanılır. Tamamı yereldir: harici API, CDN veya lisans
//! ücreti yoktur.

/// Bir sektörün tasarım kimliği. Tüm alanlar `'static`: tablo derleme
/// zamanında sabitlenir, çalışma anında bellek/belge okuma yapılmaz.
#[derive(Clone, Copy, Debug)]
pub struct SektorTasarim {
    /// Arşivdeki kayıt adı (rapor ve günlük için).
    pub ad: &'static str,
    /// Demo şablon ailesiyle uyumlu grup: "acil", "mutfak", "saglik"...
    pub grup: &'static str,
    /// Vurgu rengi (#rrggbb).
    pub vurgu: &'static str,
    /// İkincil vurgu: gradyan ve hairline parlamalarında kullanılır.
    pub vurgu2: &'static str,
    /// Sayfa zemini ve metin rengi.
    pub zemin: &'static str,
    pub yazi: &'static str,
    /// Başlık ve gövde yazı tipi yığını (gömülü fontlar).
    pub baslik: &'static str,
    pub govde: &'static str,
    /// 0: sakin kurumsal, 1: dengeli, 2: yüksek dönüşüm, 3: acil çağrı.
    pub aciliyet: u8,
    /// Hero'da gösterilecek üç rozet (kısa, kanıt odaklı).
    pub rozetler: [&'static str; 3],
    /// Birincil buton metni.
    pub cta: &'static str,
    /// Sinematik kaydırma bölümü çizilsin mi?
    pub sinematik: bool,
    /// Hareket yoğunluğu: "sakin" · "dengeli" · "canli".
    pub hareket: &'static str,
}

const VARSAYILAN: SektorTasarim = SektorTasarim {
    ad: "Genel işletme",
    grup: "genel",
    vurgu: "#2f6f4f",
    vurgu2: "#8fbf9f",
    zemin: "#0e1412",
    yazi: "#f4f1ea",
    baslik: "'Space Grotesk','Geist',system-ui,sans-serif",
    govde: "'Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
    aciliyet: 1,
    rozetler: ["Tek dokunuş", "Yerel ekip", "Hızlı dönüş"],
    cta: "İletişime geç",
    sinematik: true,
    hareket: "dengeli",
};

/// Arşivdeki sektör kimlikleri. Sıra önemlidir: ilk eşleşen kayıt kazanır.
const ARSIV: &[(&[&str], SektorTasarim)] = &[
    (
        &["cekici", "kurtarma", "yol yardim", "oto kurtarma"],
        SektorTasarim {
            ad: "Çekici & yol yardım",
            grup: "acil",
            vurgu: "#ff7a1a",
            vurgu2: "#ffd166",
            zemin: "#080d13",
            yazi: "#f6f2ea",
            baslik: "'Space Grotesk','Geist',system-ui,sans-serif",
            govde: "'Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 3,
            rozetler: [
                "7/24 aktif ekip",
                "Ortalama varış: 25 dk",
                "Canlı konum · tek dokunuş",
            ],
            cta: "Hemen ara",
            sinematik: true,
            hareket: "canli",
        },
    ),
    (
        &["cilingir", "acil servis", "acil"],
        SektorTasarim {
            ad: "Çilingir & acil servis",
            grup: "acil",
            vurgu: "#e8b04b",
            vurgu2: "#ffe1a3",
            zemin: "#0b0d10",
            yazi: "#f7f3e9",
            baslik: "'Space Grotesk','Geist',system-ui,sans-serif",
            govde: "'Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 3,
            rozetler: [
                "7/24 açık",
                "Kapıda ortalama 20 dk",
                "Konum gönder · hemen gelsin",
            ],
            cta: "Hemen ara",
            sinematik: true,
            hareket: "canli",
        },
    ),
    (
        &["restoran", "lokanta", "kebap", "menu", "mutfak"],
        SektorTasarim {
            ad: "Restoran",
            grup: "mutfak",
            vurgu: "#c2703f",
            vurgu2: "#e6b17a",
            zemin: "#120d0a",
            yazi: "#f6efe4",
            baslik: "'Cormorant Garamond','Instrument Serif',Georgia,serif",
            govde: "'Sora','Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 1,
            rozetler: [
                "Mevsimlik menü",
                "Ocağın başında 30 yıl",
                "Masa ayırt · aynı gün",
            ],
            cta: "Masa ayırt",
            sinematik: true,
            hareket: "dengeli",
        },
    ),
    (
        &["kafe", "pastane", "kahve", "tatlici"],
        SektorTasarim {
            ad: "Kafe & pastane",
            grup: "mutfak",
            vurgu: "#b08957",
            vurgu2: "#e7cfa8",
            zemin: "#12100e",
            yazi: "#f4ede2",
            baslik: "'Instrument Serif',Georgia,serif",
            govde: "'Sora','Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 1,
            rozetler: ["Günlük taze", "Kendi kavurması", "Vitrin · bugün"],
            cta: "Menüyü gör",
            sinematik: true,
            hareket: "dengeli",
        },
    ),
    (
        &["klinik", "dis", "doktor", "saglik", "hastane"],
        SektorTasarim {
            ad: "Klinik",
            grup: "saglik",
            vurgu: "#1b8fa8",
            vurgu2: "#7fd3df",
            zemin: "#f7fbfc",
            yazi: "#0e2230",
            baslik: "'Plus Jakarta Sans','Geist',system-ui,sans-serif",
            govde: "'Plus Jakarta Sans','Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 1,
            rozetler: ["Aynı gün randevu", "Uzman kadro", "Steril protokol"],
            cta: "Randevu al",
            sinematik: true,
            hareket: "sakin",
        },
    ),
    (
        &["kuafor", "berber", "guzellik", "estetik", "tirnak"],
        SektorTasarim {
            ad: "Kuaför & güzellik",
            grup: "guzellik",
            vurgu: "#c46a91",
            vurgu2: "#efb9d0",
            zemin: "#0d0b0f",
            yazi: "#f7eef4",
            baslik: "'Cormorant Garamond','Instrument Serif',Georgia,serif",
            govde: "'Sora','Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 2,
            rozetler: ["Randevuyla çalışır", "Kendi stüdyosu", "Aynı gün boşluk"],
            cta: "Randevu al",
            sinematik: true,
            hareket: "dengeli",
        },
    ),
    (
        &["otel", "pansiyon", "konaklama", "butik"],
        SektorTasarim {
            ad: "Otel",
            grup: "konaklama",
            vurgu: "#b08d57",
            vurgu2: "#e6cf9b",
            zemin: "#0e1013",
            yazi: "#f5f1e8",
            baslik: "'Cormorant Garamond','Instrument Serif',Georgia,serif",
            govde: "'Sora','Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 1,
            rozetler: ["Merkezî konum", "Kahvaltı dahil", "Erken rezervasyon"],
            cta: "Oda sor",
            sinematik: true,
            hareket: "sakin",
        },
    ),
    (
        &["emlak", "insaat", "ofis", "danisman", "avukat", "muhasebe"],
        SektorTasarim {
            ad: "Kurumsal & danışmanlık",
            grup: "kurumsal",
            vurgu: "#2a6f97",
            vurgu2: "#89c2d9",
            zemin: "#fbfbf9",
            yazi: "#16202b",
            baslik: "'Plus Jakarta Sans','Geist',system-ui,sans-serif",
            govde: "'Plus Jakarta Sans','Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 0,
            rozetler: ["Yazılı kapsam", "Aynı gün dönüş", "Gizlilik"],
            cta: "Görüşme planla",
            sinematik: true,
            hareket: "sakin",
        },
    ),
    (
        &["oto", "servis", "tamir", "lastik", "aku"],
        SektorTasarim {
            ad: "Oto servis",
            grup: "servis",
            vurgu: "#e63946",
            vurgu2: "#ff9f9f",
            zemin: "#0b1014",
            yazi: "#eef2f5",
            baslik: "'Space Grotesk','Geist',system-ui,sans-serif",
            govde: "'Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 2,
            rozetler: ["Aynı gün servis", "Şeffaf işçilik", "Parça garantisi"],
            cta: "Randevu al",
            sinematik: true,
            hareket: "canli",
        },
    ),
    (
        &["spor", "fitness", "pilates", "yoga", "salon"],
        SektorTasarim {
            ad: "Spor & fitness",
            grup: "spor",
            vurgu: "#22d3a5",
            vurgu2: "#a7f3d0",
            zemin: "#080d11",
            yazi: "#f2f7f6",
            baslik: "'Space Grotesk','Geist',system-ui,sans-serif",
            govde: "'Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 2,
            rozetler: ["Deneme dersi", "Küçük gruplar", "Esnek saatler"],
            cta: "Deneme dersi al",
            sinematik: true,
            hareket: "canli",
        },
    ),
    (
        &["egitim", "kurs", "okul", "ders", "akademi"],
        SektorTasarim {
            ad: "Eğitim",
            grup: "egitim",
            vurgu: "#3f6f4a",
            vurgu2: "#a8c9a8",
            zemin: "#fbf9f4",
            yazi: "#201a14",
            baslik: "'Instrument Serif',Georgia,serif",
            govde: "'Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 0,
            rozetler: ["Küçük sınıflar", "Deneme dersi", "Yazılı program"],
            cta: "Deneme dersi al",
            sinematik: true,
            hareket: "sakin",
        },
    ),
    (
        &["dugun", "etkinlik", "organizasyon", "salon"],
        SektorTasarim {
            ad: "Etkinlik",
            grup: "etkinlik",
            vurgu: "#a855f7",
            vurgu2: "#e9d5ff",
            zemin: "#0b0a10",
            yazi: "#f6eefb",
            baslik: "'Cormorant Garamond','Instrument Serif',Georgia,serif",
            govde: "'Geist',system-ui,-apple-system,'Segoe UI',sans-serif",
            aciliyet: 2,
            rozetler: ["Tek etkinlik", "Kendi mutfağı", "Ses & ışık dahil"],
            cta: "Tarih sor",
            sinematik: true,
            hareket: "dengeli",
        },
    ),
];

/// Metni arama için sadeleştirir: "Çilingir" ile "cilingir" eşleşir.
fn sade(metin: &str) -> String {
    metin
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
        .collect()
}

/// Sektörün tasarım kimliği; arşivde eşleşme yoksa güvenli varsayılan döner.
pub fn sec(sektor: &str) -> SektorTasarim {
    let aranan = sade(sektor);
    ARSIV
        .iter()
        .find(|(anahtarlar, _)| anahtarlar.iter().any(|anahtar| aranan.contains(anahtar)))
        .map(|(_, tasarim)| *tasarim)
        .unwrap_or(VARSAYILAN)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn cekici_acil_ve_neon_palet_alir() {
        let tasarim = sec("Çekici");
        assert_eq!(tasarim.aciliyet, 3);
        assert_eq!(tasarim.cta, "Hemen ara");
        assert!(tasarim.vurgu.starts_with('#'));
        assert!(tasarim.sinematik);
    }

    #[test]
    fn klinik_acik_zemin_alir() {
        let tasarim = sec("Diş Kliniği");
        assert_eq!(tasarim.grup, "saglik");
        assert!(tasarim.zemin.starts_with("#f"));
    }

    #[test]
    fn bilinmeyen_sektor_varsayilana_duser() {
        let tasarim = sec("Uzay İstasyonu");
        assert_eq!(tasarim.ad, VARSAYILAN.ad);
        assert_eq!(tasarim.aciliyet, VARSAYILAN.aciliyet);
    }
}
