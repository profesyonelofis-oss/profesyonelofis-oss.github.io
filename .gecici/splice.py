from pathlib import Path

kok = Path('.')

# --- stok_video.rs: eski API kitaplığını yerel içe aktarmayla değiştir -----
p = kok / 'src/stok_video.rs'
s = p.read_text(encoding='utf-8')
if not s.startswith('#![allow(dead_code)]'):
    s = '#![allow(dead_code)]\n' + s
bas = s.index('// --- Kitaplık')
son = s.index('#[cfg(test)]\nmod testler {', bas)
yeni = (kok / '.gecici/yeni_kitaplik.rs').read_text(encoding='utf-8')
s = s[:bas] + yeni + s[son:]
s = s.replace(
    'pub struct Hazir {\n    pub grup: String,\n',
    'pub struct Hazir {\n    pub grup: String,\n    /// Klasör içindeki dosya adı (`v2.mp4`).\n    pub dosya_adi: String,\n    /// Klasör içindeki poster adı (`v2.webp`).\n    pub poster_adi: String,\n',
    1,
)
s = s.replace(
    'pub struct VideoBilgi {\n    pub grup: String,\n',
    'pub struct VideoBilgi {\n    pub grup: String,\n    /// Klasör içindeki dosya adı (`v2.mp4`).\n    pub dosya_adi: String,\n    /// Klasör içindeki poster adı (`v2.webp`).\n    pub poster_adi: String,\n',
    1,
)
p.write_text(s, encoding='utf-8')

# --- stok_kasa.rs: kullanılmayan kasaya izin (API kapalı) ------------------
p = kok / 'src/stok_kasa.rs'
s = p.read_text(encoding='utf-8')
if not s.startswith('#![allow(dead_code)]'):
    s = '#![allow(dead_code)]\n' + s
p.write_text(s, encoding='utf-8')

# --- main.rs: alanlar, başlangıç değerleri, panel -------------------------
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

a = s.index('/// Stok video arka plan işinden gelen olaylar.')
b = s.index('#[cfg(test)]\nmod testler {', a)
panel = (kok / '.gecici/yeni_panel.rs').read_text(encoding='utf-8')
s = s[:a] + panel + '\n' + s[b:]

# Demo girdisi: tohumla video seçimi.
s = s.replace(
    'stok_video::video_bilgisi(grup.id)',
    'stok_video::sec(grup.id, stok_video::tohum(&isletme.name, &isletme.address))',
)

# Klasör çıktısı: seçilen kaydın kendi dosya adları kopyalanır.
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
if eski not in s:
    raise SystemExit('kopyalama bloğu bulunamadı')
s = s.replace(eski, yeni_kopya, 1)
p.write_text(s, encoding='utf-8')

print('splice tamam')
