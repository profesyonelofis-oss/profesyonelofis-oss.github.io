import sys

def oku(p):
    return open(p, encoding='utf-8').read()

def yaz(p, s):
    open(p, 'w', encoding='utf-8', newline='\n').write(s)

p = 'src/google.rs'
s = oku(p)

eski_imza = "fn isle(yanit: &Yanit) -> Result<(Vec<BizKayit>, usize), String> {"
yeni_imza = '''fn isle(yanit: &Yanit) -> Result<(Vec<BizKayit>, usize), String> {
    isle_ile(yanit, true)
}

/// `site_ele = false`: web sitesi olan kayıtlar da döner. Tek işletme
/// aramasında işletmenin sitesi olsa da koordinatı gerekir.
fn isle_ile(yanit: &Yanit, site_ele: bool) -> Result<(Vec<BizKayit>, usize), String> {'''
if eski_imza not in s:
    sys.exit("BULUNAMADI: isle imzasi")
s = s.replace(eski_imza, yeni_imza, 1)

eski_site = """        if !site.is_empty() {
            siteli += 1;
            continue;
        }"""
yeni_site = """        if !site.is_empty() {
            siteli += 1;
            if site_ele {
                continue;
            }
        }"""
if eski_site not in s:
    sys.exit("BULUNAMADI: site eleme")
s = s.replace(eski_site, yeni_site, 1)

eski_cagri = "        let (liste, _) = isle(&yanit)?;"
yeni_cagri = "        let (liste, _) = isle_ile(&yanit, false)?;"
if eski_cagri not in s:
    sys.exit("BULUNAMADI: ad_ara cagrisi")
s = s.replace(eski_cagri, yeni_cagri, 1)

eski_hata = """        liste
            .into_iter()
            .next()
            .ok_or_else(|| format!("'{ad}' Google kayıtlarında bulunamadı."))"""
yeni_hata = """        liste.into_iter().next().ok_or_else(|| {
            format!(
                "'{ad}' Google kayıtlarında bulunamadı ({} ham kayıt).",
                yanit.places.len()
            )
        })"""
if eski_hata not in s:
    sys.exit("BULUNAMADI: ad_ara hata metni")
s = s.replace(eski_hata, yeni_hata, 1)
yaz(p, s)
print("degisti: google.rs isle_ile + site filtresi kapatildi")
print("TAMAM")
