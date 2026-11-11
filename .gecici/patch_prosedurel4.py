from pathlib import Path

p = Path('src/demo_site.rs')
s = p.read_text(encoding='utf-8')

eski = '''    let sayi = u64::from_str_radix(tohum.trim(), 16).unwrap_or(0x5eed_1234);
    let aci = (sayi % 360) as u32;
    let aci2 = ((sayi >> 7) % 360) as u32;
    let hiz = 0.55 + ((sayi >> 13) % 100) as f64 / 100.0 * 0.9;
    let dalga = 0.72 + ((sayi >> 21) % 100) as f64 / 100.0 * 0.86;
    let parca = 0.16 + ((sayi >> 29) % 100) as f64 / 100.0 * 0.62;
    let vinyet = 0.34 + ((sayi >> 37) % 100) as f64 / 100.0 * 0.32;'''
yeni = '''    let sayi = u64::from_str_radix(tohum.trim(), 16).unwrap_or(0x5eed_1234);
    // Tohum yalnız 24 bit: bitleri karıştırıp üç bağımsız akışa ayırırız.
    let a = tohum_karistir(sayi);
    let b = tohum_karistir(a);
    let c = tohum_karistir(b);
    let aci = (a % 360) as u32;
    let aci2 = ((a >> 9) % 360) as u32;
    let hiz = 0.55 + (b % 100) as f64 / 100.0 * 0.9;
    let dalga = 0.72 + ((b >> 8) % 100) as f64 / 100.0 * 0.86;
    let parca = 0.16 + (c % 100) as f64 / 100.0 * 0.62;
    let vinyet = 0.34 + ((c >> 8) % 100) as f64 / 100.0 * 0.32;'''
assert eski in s
s = s.replace(eski, yeni, 1)

eski_fn = '/// `#rrggbb` (veya `#rgb`) rengi 0–1 aralığına çevirir.'
yeni_fn = '''/// 64 bit karıştırıcı (SplitMix64): küçük tohumlarda bit dağılımı düzgün olur.
fn tohum_karistir(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// `#rrggbb` (veya `#rgb`) rengi 0–1 aralığına çevirir.'''
assert eski_fn in s
s = s.replace(eski_fn, yeni_fn, 1)

eski_test = '''        assert!(a.parca >= 0.16 && a.parca <= 0.78);'''
yeni_test = '''        assert!(a.parca >= 0.16 && a.parca <= 0.78);
        assert_ne!(a.parca, c.parca, "partikül yoğunluğu değişmeli");
        assert_ne!(a.vinyet, c.vinyet, "vinyet değişmeli");'''
assert eski_test in s
s = s.replace(eski_test, yeni_test, 1)
p.write_text(s, encoding='utf-8')
print('tohum karıştırıcı eklendi')
