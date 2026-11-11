from pathlib import Path

kok = Path('.')

# --- stok_video.rs düzeltmeleri -------------------------------------------
p = kok / 'src/stok_video.rs'
s = p.read_text(encoding='utf-8')

# API yolundaki Hazir başlatıcısı yeni alanları taşısın.
s = s.replace(
    "    Ok(Hazir {\n        grup: grup_id.to_string(),\n        video_bayt,",
    "    Ok(Hazir {\n        grup: grup_id.to_string(),\n        dosya_adi: \"hero.mp4\".to_string(),\n        poster_adi: \"poster.webp\".to_string(),\n        video_bayt,",
    1,
)

# Testlerin kullanacağı yol verilebilir okuma yardımcıları.
s = s.replace(
    "pub fn kayitlar(grup_id: &str) -> Vec<VideoBilgi> {\n    let klasor = Path::new(MEDYA_KOK).join(grup_id);\n    let json = kaynak_oku(&klasor);",
    "pub fn kayitlar(grup_id: &str) -> Vec<VideoBilgi> {\n    kayitlar_yoldan(&Path::new(MEDYA_KOK).join(grup_id), grup_id)\n}\n\n/// Yol verilebilir kayıt okuma (testler ve taşınabilir kullanım).\npub fn kayitlar_yoldan(klasor: &Path, grup_id: &str) -> Vec<VideoBilgi> {\n    let json = kaynak_oku(klasor);",
    1,
)
s = s.replace(
    "    kayit_listesi(&json)\n        .iter()\n        .filter_map(|kayit| bilgi_uret(&klasor, grup_id, kayit))\n        .collect()\n}",
    "    kayit_listesi(&json)\n        .iter()\n        .filter_map(|kayit| bilgi_uret(klasor, grup_id, kayit))\n        .collect()\n}",
    1,
)
s = s.replace(
    "pub fn sec(grup_id: &str, tohum: u64) -> Option<VideoBilgi> {\n    let liste = kayitlar(grup_id);",
    "pub fn sec(grup_id: &str, tohum: u64) -> Option<VideoBilgi> {\n    sec_yoldan(&Path::new(MEDYA_KOK).join(grup_id), grup_id, tohum)\n}\n\n/// Yol verilebilir seçim: aynı sektörde tohum mod kayıt sayısı.\npub fn sec_yoldan(klasor: &Path, grup_id: &str, tohum: u64) -> Option<VideoBilgi> {\n    let liste = kayitlar_yoldan(klasor, grup_id);",
    1,
)

# Eski testler (kaldırılan video_bilgisi_yoldan'a bağlıydı) yerine yeni testler.
eski_test_bas = s.index('    #[test]\n    fn kaynak_dosyasi_yazilip_geri_okunur()')
s = s[:eski_test_bas] + '''    fn gecici_klasor(ad: &str) -> std::path::PathBuf {
        let klasor = std::env::temp_dir().join(format!("nexus_yerel_{}_{ad}", std::process::id()));
        let _ = std::fs::create_dir_all(&klasor);
        klasor
    }

    #[test]
    fn uzanti_kontrolu_mp4_mov_webm_kabul_eder() {
        assert!(uzanti_gecerli("kuafor.mp4"));
        assert!(uzanti_gecerli("KUAfor.MOV"));
        assert!(uzanti_gecerli("tanitim.webm"));
        assert!(!uzanti_gecerli("eski.avi"));
        assert!(!uzanti_gecerli("uzantisiz"));
    }

    #[test]
    fn baslangic_video_sinirina_cekilir() {
        assert_eq!(baslangic_sinirla(-3.0, 20.0), 0.0);
        assert_eq!(baslangic_sinirla(2.0, 20.0), 2.0);
        assert_eq!(baslangic_sinirla(18.0, 20.0), 12.0);
        assert_eq!(baslangic_sinirla(4.0, 5.0), 0.0);
    }

    #[test]
    fn coklu_video_kaydi_okunur_ve_tohumla_secilir() {
        let klasor = gecici_klasor("coklu");
        for ad in ["v1.mp4", "v2.mp4"] {
            std::fs::write(klasor.join(ad), [0u8; 16]).expect("video");
        }
        for ad in ["v1.webp", "v2.webp"] {
            std::fs::write(klasor.join(ad), [1u8; 8]).expect("poster");
        }
        let json = serde_json::json!({"surum": 2, "videolar": [
            {"dosya": "v1.mp4", "poster": "v1.webp", "atif": "A · Lisans", "atif_gerekli": true},
            {"dosya": "v2.mp4", "poster": "v2.webp", "atif": "B", "atif_gerekli": false}
        ]});
        kaynak_yaz(&klasor, &json).expect("KAYNAK.json");
        assert_eq!(kayitlar_yoldan(&klasor, "kuafor").len(), 2);
        let a = sec_yoldan(&klasor, "kuafor", 0).expect("ilk");
        let b = sec_yoldan(&klasor, "kuafor", 1).expect("ikinci");
        assert_ne!(a.dosya_adi, b.dosya_adi, "komşu işletme farklı video almalı");
        assert_eq!(a.dosya, "media/hero.mp4");
        assert_eq!(a.poster, "media/poster.webp");
        assert!(a.sayfa_atfi().is_some(), "atıf istenmişse altbilgiye girer");
        assert!(b.sayfa_atfi().is_none(), "atıf istenmemişse girmez");
        let _ = std::fs::remove_dir_all(&klasor);
    }

    #[test]
    fn eski_tek_video_bicimi_de_okunur() {
        let klasor = gecici_klasor("eski");
        std::fs::write(klasor.join("hero.mp4"), [0u8; 16]).expect("video");
        std::fs::write(klasor.join("poster.webp"), [1u8; 8]).expect("poster");
        let json = serde_json::json!({"dosya": "hero.mp4", "poster": "poster.webp"});
        kaynak_yaz(&klasor, &json).expect("KAYNAK.json");
        let liste = kayitlar_yoldan(&klasor, "kuafor");
        assert_eq!(liste.len(), 1);
        assert_eq!(liste[0].dosya_adi, "hero.mp4");
        let _ = std::fs::remove_dir_all(&klasor);
    }

    #[test]
    fn bozuk_dosya_turkce_hata_verir() {
        let klasor = gecici_klasor("bozuk");
        let yol = klasor.join("bozuk.mp4");
        std::fs::write(&yol, b"bu bir video degil").expect("dosya");
        let istek = YerelIstek {
            kaynak_yol: yol.to_string_lossy().to_string(),
            baslangic: 0.0,
            kaynak_linki: String::new(),
            cekim: String::new(),
            lisans: String::new(),
            atif_gerekli: false,
        };
        let hata = match yerel_hazirla("kuafor", &istek) {
            Ok(_) => panic!("bozuk dosya kabul edilmemeli"),
            Err(hata) => hata,
        };
        assert!(
            hata.contains("okunamadı") || hata.contains("dönüştürülemedi"),
            "sade Türkçe hata bekleniyordu: {hata}"
        );
        let _ = std::fs::remove_dir_all(&klasor);
    }
}
'''
p.write_text(s, encoding='utf-8')

# --- main.rs: alanlar, başlangıç değerleri, panel, demo seçimi ------------
p = kok / 'src/main.rs'
s = p.read_text(encoding='utf-8')

a = s.index('    stok_grup: String,')
b = s.index('    stok_mesgul: bool,', a) + len('    stok_mesgul: bool,\n')
alanlar = '''    /// Stok video bulucu (yerel içe aktarma): seçili grup, dosya yolu,
    /// kırpma başlangıcı, künye alanları ve arka plan işinin kanalı.
    stok_grup: String,
    stok_yol: String,
    stok_baslangic: String,
    stok_linki: String,
    stok_cekim: String,
    stok_lisans: String,
    stok_atif_goster: bool,
    stok_durum: String,
    stok_kutuphane: Vec<stok_video::VideoBilgi>,
    stok_islem: Option<Receiver<StokOlay>>,
    stok_mesgul: bool,
'''
s = s[:a] + alanlar + s[b:]

a = s.index('            stok_grup:')
b = s.index('            stok_mesgul: false,', a) + len('            stok_mesgul: false,\n')
init = '''            stok_grup: stok_video::GRUPLAR[0].id.to_string(),
            stok_yol: String::new(),
            stok_baslangic: "0".to_string(),
            stok_linki: String::new(),
            stok_cekim: String::new(),
            stok_lisans: String::new(),
            stok_atif_goster: false,
            stok_durum: String::new(),
            stok_kutuphane: stok_video::kutuphane(),
            stok_islem: None,
            stok_mesgul: false,
'''
s = s[:a] + init + s[b:]

# Panel bloğu: enum StokOlay'dan testlerin başına kadar.
pano = s.index('fn stok_paneli')
bas = s.rfind('enum StokOlay', 0, pano)
# Enum'un üstündeki açıklama satırını da kes.
satir_basi = s.rfind('\n', 0, bas)
if s[satir_basi + 1 : satir_basi + 4] == '///':
    bas = satir_basi + 1
son = s.index('#[cfg(test)]\nmod testler {', pano)
panel = (kok / '.gecici/yeni_panel.rs').read_text(encoding='utf-8')
s = s[:bas] + panel + '\n' + s[son:]

s = s.replace(
    'stok_video::video_bilgisi(grup.id)',
    'stok_video::sec(grup.id, stok_video::tohum(&isletme.name, &isletme.address))',
)

eski = '''        if let Some(grup) = stok_video::grup_esle(&isletme.category) {
            let kaynak = format!("{}/{}", stok_video::MEDYA_KOK, grup.id);
            for ad in ["hero.mp4", "poster.webp", "KAYNAK.json"] {
                let _ = fs::copy(format!("{kaynak}/{ad}"), format!("{medya}/{ad}"));
            }
        }'''
yeni_kopya = '''        if let Some(grup) = stok_video::grup_esle(&isletme.category) {
            let kaynak = format!("{}/{}", stok_video::MEDYA_KOK, grup.id);
            // Tohumla seçilen kaydın dosyaları sayfaya hero/poster olarak kopyalanır.
            if let Some(bilgi) = stok_video::sec(
                grup.id,
                stok_video::tohum(&isletme.name, &isletme.address),
            ) {
                let _ = fs::copy(
                    format!("{kaynak}/{}", bilgi.dosya_adi),
                    format!("{medya}/hero.mp4"),
                );
                let _ = fs::copy(
                    format!("{kaynak}/{}", bilgi.poster_adi),
                    format!("{medya}/poster.webp"),
                );
            }
            let _ = fs::copy(
                format!("{kaynak}/KAYNAK.json"),
                format!("{medya}/KAYNAK.json"),
            );
        }'''
if eski in s:
    s = s.replace(eski, yeni_kopya, 1)
p.write_text(s, encoding='utf-8')

print('splice v2 tamam')
