import io

p = "src/demo_site.rs"
s = io.open(p, encoding="utf-8", newline="").read().replace("\r\n", "\n")

# 1) Acil sektor yardimcisi
isaret = "/// Rengin algılanan parlaklığı; açık temada gölge/karartma yönü değişir."
yardimci = '''/// Acil çağrı sektörleri (çekici, yol yardım, çilingir, acil servis): sayfada
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

'''
assert isaret in s
s = s.replace(isaret, yardimci + isaret, 1)

# 2) uret icinde acil degiskenleri
eski = '''    let bolum_metni = ozel(&g.metin.bolum_baslik, ic.bolum);
'''
yeni = '''    let bolum_metni = ozel(&g.metin.bolum_baslik, ic.bolum);

    // Acil sektörler: konum gönderme, canlı rozetler ve yol yardım asistanı.
    let acil = acil_sektor(&g.sektor);
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
        format!(
            r##"<div class="hero-rozetler"><span class="hcip canli"><b></b>7/24 Aktif {sektor} Ekibi</span><span class="hcip">Ortalama Varış: 15–20 Dk</span><span class="hcip">Konum · tek dokunuş</span></div>"##
        )
    } else {
        String::new()
    };
'''
assert eski in s
s = s.replace(eski, yeni, 1)

# 3) Klasik hero: rozetler + konum dugmesi
eski = '''<p class="hero-alt">{alt}</p>
<div class="hero-butonlar">{birincil}{ikincil}</div>'''
yeni = '''<p class="hero-alt">{alt}</p>
{hero_rozetler}
<div class="hero-butonlar">{birincil}{ikincil}{konum_buton}</div>'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''            alt = kacis(&slogan),
            birincil = birincil,
            ikincil = ikincil,
'''
yeni = '''            alt = kacis(&slogan),
            birincil = birincil,
            ikincil = ikincil,
            hero_rozetler = hero_rozetler,
            konum_buton = konum_buton,
'''
assert eski in s
s = s.replace(eski, yeni, 1)

# 4) Sahne hero: rozetler + konum dugmesi
eski = '''<p class="hero-alt">{alt}</p>
<div class="hero-butonlar">{birincil}<a class="buton buton-cizgi" href="#menu">Menüyü gör</a></div>'''
yeni = '''<p class="hero-alt">{alt}</p>
{hero_rozetler}
<div class="hero-butonlar">{birincil}<a class="buton buton-cizgi" href="#menu">Menüyü gör</a>{konum_buton}</div>'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''            alt = kacis(&slogan),
            birincil = birincil,
            hikaye = hikaye,'''
yeni = '''            alt = kacis(&slogan),
            birincil = birincil,
            hero_rozetler = hero_rozetler,
            konum_buton = konum_buton,
            hikaye = hikaye,'''
assert eski in s
s = s.replace(eski, yeni, 1)

# 5) Mobil CTA: acil sektorde ara + canli konum bari
eski = '''    let mobil_cta = match &tel_hedef {
        Some(hedef) => format!(
            r##"<a class="mobil-cta" href="{hedef}">{}</a>"##,
            kacis(&cta)
        ),
        None => String::new(),
    };'''
yeni = '''    let mobil_cta = if acil && !wa_no.is_empty() {
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
    };'''
assert eski in s
s = s.replace(eski, yeni, 1)

# 6) Yol yardim asistani
eski = '''    // Demo süresi: ilk açılıştan 12 saat sonra sayfa kapanır.'''
yeni = '''    let asistan = if acil && !wa_no.is_empty() {
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

    // Demo süresi: ilk açılıştan 12 saat sonra sayfa kapanır.'''
assert eski in s
s = s.replace(eski, yeni, 1)

# 7) Yorumlar: kaydirici
eski = '''<div class="yorumlar">{yorumlar}</div>
<p class="ornek-not">Yorumlar örnektir · gerçek yorumlarla değiştirilecek</p>'''
yeni = '''<div class="yorum-kaydirici" id="yorum-kaydirici">
<div class="yorum-iz" id="yorum-iz">{yorumlar}</div>
<div class="yorum-kontroller">
<button class="yk-btn" type="button" data-yon="-1" aria-label="Önceki yorum">‹</button>
<button class="yk-btn" type="button" data-yon="1" aria-label="Sonraki yorum">›</button>
<div class="yorum-noktalar" id="yorum-noktalar"></div>
</div>
</div>
<p class="ornek-not">Yorumlar örnektir · gerçek yorumlarla değiştirilecek</p>'''
assert eski in s
s = s.replace(eski, yeni, 1)

# 8) Stil ve betik katmani
eski = '''    // JS kapalıysa içerik görünür kalır: açılım animasyonu beklemesin.'''
yeni = '''    sayfa.push_str(EK_CSS);
    // JS kapalıysa içerik görünür kalır: açılım animasyonu beklemesin.'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''    sayfa.push_str(&SCRIPTS.replace("__PROGRAM__", &program));'''
yeni = '''    sayfa.push_str(&SCRIPTS.replace("__PROGRAM__", &program));
    sayfa.push_str(&EK_JS.replace("__WA__", &wa_no));'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski = '''    sayfa.push_str(
        "<div class=\\"suresi-doldu\\" id=\\"suresi-doldu\\" hidden>\\n<p>Bu demonun süresi doldu.<br>Güncel bir demo için işletmeyle iletişime geçin.</p>\\n</div>\\n",
    );'''
yeni = eski + '''
    sayfa.push_str(&asistan);'''
assert eski in s
s = s.replace(eski, yeni, 1)

io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("demo_site yamalandi")
