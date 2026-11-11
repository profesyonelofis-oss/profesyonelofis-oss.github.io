import re, subprocess, glob, os, sys, collections

SRC = glob.glob('src/**/*.rs', recursive=True)
BS = chr(92)


def oku(p):
    return open(p, encoding='utf-8').read()


def yaz(p, s):
    open(p, 'w', encoding='utf-8', newline='\n').write(s)


def testte_kullaniliyor(ad):
    desen = re.compile(r'\b' + ad + r'\b')
    for d in SRC:
        s = oku(d)
        i = s.find('#[cfg(test)]')
        if i < 0:
            continue
        if desen.search(s[i:]):
            return True
    return False


def govde_sil(p, ad, anahtar_kelime):
    s = oku(p)
    i = s.find(anahtar_kelime + " " + ad)
    if i < 0:
        return False
    bas = s.rfind("\n", 0, i) + 1
    while True:
        o = s.rfind("\n", 0, bas - 1)
        if o < 0:
            break
        satir = s[o + 1:bas].strip()
        if satir.startswith(("///", "//", "#[")):
            bas = o + 1
        else:
            break
    j = s.find("{", i)
    if j < 0:
        return False
    d = 0
    while j < len(s):
        if s[j] == '{':
            d += 1
        elif s[j] == '}':
            d -= 1
            if d == 0:
                break
        j += 1
    son = j + 1
    if s[son:son + 1] == ';':
        son += 1
    son = s.find("\n", son) + 1
    yaz(p, s[:bas] + s[son:])
    return True


def uyarilari_al():
    for d in SRC:
        os.utime(d, None)
    cikti = subprocess.run(["cargo", "build", "--release", "--bins"],
                           capture_output=True, text=True, errors="replace").stderr
    satirlar = cikti.splitlines()
    hedef = []
    rapor = collections.defaultdict(list)
    w = None
    for k, satir in enumerate(satirlar):
        if satir.startswith("warning: "):
            w = satir[9:]
        m = re.search(r'--> (src[\\/][^:]+):\d+', satir)
        if not (m and w):
            continue
        dosya = m.group(1).replace(BS, "/")
        rapor[dosya].append(w)
        adlar, kelime = [], None
        m1 = re.search(r'^(function|constant|static|struct|enum|type alias) `([a-z0-9_]+)` is never used', w)
        if m1:
            kelime, adlar = m1.group(1), [m1.group(2)]
        m2 = re.search(r'^(?:associated function|method) `([a-z0-9_]+)` is never used', w)
        if m2:
            kelime, adlar = "fn", [m2.group(1)]
        m3 = re.search(r'^methods `([^`]+)` are never used', w)
        if m3:
            kelime, adlar = "fn", re.findall(r'`([a-z0-9_]+)`', m3.group(1))
        if m1 and kelime == "constant":
            kelime = "const"
        if m1 and kelime == "static":
            kelime = "static"
        if kelime == "type alias":
            kelime = "type"
        if kelime and adlar:
            for ad in adlar:
                hedef.append((dosya, ad, kelime))
    return hedef, rapor, cikti


def temizle(tur_sayisi=6):
    for tur in range(tur_sayisi):
        hedefler, rapor, _ = uyarilari_al()
        if not hedefler:
            print("tur", tur, ": silinecek hedef yok")
            return rapor
        sayi = 0
        for dosya, ad, kelime in sorted(set(hedefler)):
            if testte_kullaniliyor(ad):
                print("atlandi (testte kullaniliyor):", ad)
                continue
            if govde_sil(dosya, ad, kelime):
                sayi += 1
                print("silindi:", dosya, ad)
        print("tur", tur, ": ", sayi, "oge silindi")
        if sayi == 0:
            return rapor
    return rapor


if __name__ == "__main__":
    if "--sil" in sys.argv:
        rapor = temizle()
    else:
        _, rapor, _ = uyarilari_al()
    for dosya in sorted(rapor, key=lambda d: -len(rapor[d])):
        print("== %s (%d)" % (dosya, len(rapor[dosya])))
        for x in rapor[dosya]:
            print("   ", x)
