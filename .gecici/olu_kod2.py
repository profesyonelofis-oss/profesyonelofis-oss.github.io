import json, subprocess, glob, os, re, sys, collections

SRC = sorted(glob.glob('src/**/*.rs', recursive=True))
TEXT = {p: open(p, encoding='utf-8').read() for p in SRC}
BS = chr(92)


def taze():
    for d in SRC:
        os.utime(d, None)


def tani():
    taze()
    cikti = subprocess.run(["cargo", "build", "--release", "--bins",
                            "--message-format=json"],
                           capture_output=True, text=True, errors="replace")
    hedef, rapor = [], collections.defaultdict(list)
    for satir in cikti.stdout.splitlines():
        try:
            kayit = json.loads(satir)
        except Exception:
            continue
        if kayit.get('reason') != 'compiler-message':
            continue
        m = kayit.get('message') or {}
        if m.get('level') != 'warning':
            continue
        metin = m.get('message', '')
        kod = (m.get('code') or {}).get('code') if isinstance(m.get('code'), dict) else None
        if kod != 'dead_code':
            continue
        span = None
        for s in m.get('spans') or []:
            if s.get('is_primary'):
                span = s
                break
        if not span:
            continue
        dosya = span['file_name'].replace(BS, '/')
        adlar = []
        kelime = None
        m1 = re.search(r'^(function|constant|static) `([A-Za-z0-9_]+)` is never used', metin)
        if m1:
            kelime, adlar = m1.group(1), [m1.group(2)]
        m2 = re.search(r'^(?:associated function|method) `([A-Za-z0-9_]+)` is never used', metin)
        if m2:
            kelime, adlar = "fn", [m2.group(1)]
        m3 = re.search(r'^methods `([^`]+)` are never used', metin)
        if m3:
            kelime, adlar = "fn", re.findall(r'`([A-Za-z0-9_]+)`', m3.group(1))
        if kelime:
            kelime = {"constant": "const", "static": "static", "function": "fn"}.get(kelime, kelime)
            for ad in adlar:
                hedef.append((dosya, ad, kelime, span.get('byte_start'), span.get('byte_end')))
        else:
            rapor[dosya].append(metin)
    return hedef, rapor


def test_bolgesi(s):
    i = s.find('#[cfg(test)]')
    return i if i >= 0 else len(s)


def disarida_kullanim(dosya, ad, bas, son):
    # Güvenli kural: ad yalnızca tanımında geçiyorsa (testler dahil hiçbir yerde
    # çağrılmıyorsa) ölü kabul et. Test kullanan yardımcılara dokunulmaz.
    desen = re.compile(r'\b' + ad + r'\b')
    for p, s in TEXT.items():
        for m in desen.finditer(s):
            if p == dosya and bas is not None and bas <= m.start() <= son:
                continue
            return True
    return False


def oge_sil(dosya, ad, kelime, bas, son):
    s = TEXT[dosya]
    i = s.rfind("\n", 0, bas) + 1
    # Belgenin ilişik yorum/öznitelik satırları varsa yukarı doğru genişlet.
    while True:
        o = s.rfind("\n", 0, i - 1)
        if o < 0:
            break
        satir = s[o + 1:i].strip()
        if satir.startswith(("///", "//", "#[")):
            i = o + 1
        else:
            break
    j = s.find("{", bas)
    nokta = s.find(";", bas)
    if j < 0 or (0 <= nokta < j):
        k = s.find("\n", nokta) + 1 if nokta >= 0 else s.find("\n", bas) + 1
    else:
        d = 0
        while j < len(s):
            if s[j] == '{':
                d += 1
            elif s[j] == '}':
                d -= 1
                if d == 0:
                    break
            j += 1
        k = s.find("\n", j) + 1
    metin = s[:i] + s[k:]
    open(dosya, 'w', encoding='utf-8', newline='\n').write(metin)
    TEXT[dosya] = metin
    return True


def temizle(tur=10):
    for t in range(tur):
        hedef, rapor = tani()
        if not hedef:
            return rapor
        sayi = 0
        for dosya, ad, kelime, bas, son in hedef:
            if bas is None or son is None:
                continue
            if disarida_kullanim(dosya, ad, bas, son):
                continue
            if oge_sil(dosya, ad, kelime, bas, son):
                sayi += 1
                print("silindi:", dosya, ad)
        print("tur", t, "->", sayi)
        if sayi == 0:
            return rapor
    return rapor


if __name__ == '__main__':
    if '--sil' in sys.argv:
        rapor = temizle()
    else:
        _, rapor = tani()
    toplam = sum(len(v) for v in rapor.values())
    print("kalan dead_code uyarisi:", toplam)
    for d in sorted(rapor, key=lambda x: -len(rapor[x])):
        print("==", d, len(rapor[d]))
        for x in rapor[d][:10]:
            print("   ", x)
