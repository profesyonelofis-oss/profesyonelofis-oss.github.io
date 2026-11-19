import io, json

# --- 1) kaynak.json: cekici gorsellerinin atfi -------------------------
kp = "varliklar/foto/kaynak.json"
veri = json.load(io.open(kp, encoding="utf-8"))
veri["cekici"] = [
    {
        "dosya": "1.svg",
        "atif": "Nexus Enterprise · özgün çizim · CC0",
        "kaynak": "sector-matched illustration (tow truck at dusk)",
    },
    {
        "dosya": "2.svg",
        "atif": "Nexus Enterprise · özgün çizim · CC0",
        "kaynak": "sector-matched illustration (roadside assistance)",
    },
    {
        "dosya": "3.svg",
        "atif": "Nexus Enterprise · özgün çizim · CC0",
        "kaynak": "sector-matched illustration (night recovery)",
    },
]
io.open(kp, "w", encoding="utf-8", newline="\n").write(
    json.dumps(veri, ensure_ascii=False, indent=2) + "\n"
)

# --- 2) main.rs: stok ailesi + mime + hero yedegi ----------------------
p = "src/main.rs"
s = io.open(p, encoding="utf-8", newline="").read().replace("\r\n", "\n")

eski = '''        "eglence" => vec!['''
yeni = '''        "cekici" => vec![
            include_bytes!("../varliklar/foto/cekici/1.svg").as_slice(),
            include_bytes!("../varliklar/foto/cekici/2.svg").as_slice(),
            include_bytes!("../varliklar/foto/cekici/3.svg").as_slice(),
        ],
        "eglence" => vec!['''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''/// Galeriye gömülecek örnek fotoğraflar (mime, base64).
fn stok_galeri(aile: &str) -> Vec<(String, String)> {
    stok_dosyalar(aile)
        .iter()
        .map(|veri| ("image/webp".to_string(), base64_kodla(veri)))
        .collect()
}'''
yeni = '''/// Gömülü karelerin MIME türü: çekici ailesi özgün SVG çizimlerdir,
/// diğer aileler WebP fotoğraflardır.
fn stok_mime(aile: &str) -> &'static str {
    if aile == "cekici" {
        "image/svg+xml"
    } else {
        "image/webp"
    }
}

/// Galeriye gömülecek örnek fotoğraflar (mime, base64).
fn stok_galeri(aile: &str) -> Vec<(String, String)> {
    let mime = stok_mime(aile);
    stok_dosyalar(aile)
        .iter()
        .map(|veri| (mime.to_string(), base64_kodla(veri)))
        .collect()
}'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''fn demo_turu_adi(sektor: &str) -> &'static str {'''
yeni = '''/// Çekici / kurtarma / yol yardım sektörleri kendi görsel ailesini kullanır:
/// oto tamirhanesi değil, doğrudan çekici ve kurtarıcı kareleri.
fn stok_ailesi(sektor: &str) -> &'static str {
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
    if ["cekici", "kurtarma", "yol yardim"]
        .iter()
        .any(|anahtar| sade.contains(anahtar))
    {
        "cekici"
    } else {
        demo_turu_adi(sektor)
    }
}

fn demo_turu_adi(sektor: &str) -> &'static str {'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''    let aile = demo_turu_adi(&isletme.category);
    (stok_galeri(aile), true, stok_atif(aile))'''
yeni = '''    let aile = stok_ailesi(&isletme.category);
    (stok_galeri(aile), true, stok_atif(aile))'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''    let (foto, foto_atif) = hero_fotografi(&ayar, oto_foto, oto_foto_atif, &ek_fotolar);'''
yeni = '''    let (foto, foto_atif) = hero_fotografi(&ayar, oto_foto, oto_foto_atif, &ek_fotolar);
    // İşletmenin kendi fotoğrafı yoksa hero'ya sektörle birebir eşleşen
    // gömülü kare gelir: çekicide çekici, kafede kahve, klinikte muayene.
    let (foto, foto_atif) = if foto.is_none() && !temiz {
        let aile = stok_ailesi(&isletme.category);
        match stok_galeri(aile).into_iter().next() {
            Some((mime, veri)) => (Some((mime, veri)), stok_atif(aile)),
            None => (foto, foto_atif),
        }
    } else {
        (foto, foto_atif)
    };'''
assert eski in s
s = s.replace(eski, yeni, 1)

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("stok yamalandi")
