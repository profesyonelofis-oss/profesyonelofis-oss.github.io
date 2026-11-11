import re, subprocess, glob, os, collections

SRC = sorted(glob.glob('src/**/*.rs', recursive=True))


def oku(p):
    return open(p, encoding='utf-8').read()


def yaz(p, s):
    open(p, 'w', encoding='utf-8', newline='\n').write(s)


def tani():
    for d in SRC:
        os.utime(d, None)
    cikti = subprocess.run(["cargo", "build", "--release", "--bins"],
                           capture_output=True, text=True, errors="replace").stderr
    satirlar = cikti.splitlines()
    hedef, kalan = [], collections.Counter()
    w = None
    for k, satir in enumerate(satirlar):
        if satir.startswith('warning: '):
            w = satir[9:]
        m = re.search(r'--> (src[\\/][^:]+):\d+', satir)
        if not (m and w):
            continue
        dosya = m.group(1).replace(chr(92), '/')
        kelime, adlar = None, []
        for desen, anahtar in (
            (r'^function `([A-Za-z0-9_]+)` is never used', 'fn'),
            (r'^constant `([A-Za-z0-9_]+)` is never used', 'const'),
            (r'^(?:associated function|method) `([A-Za-z0-9_]+)` is never used', 'fn'),
        ):
            mm = re.search(desen, w)
            if mm:
                kelime, adlar = anahtar, [mm.group(1)]
                break
        mm = re.search(r'^methods `([^`]+)` are never used', w)
        if mm:
            kelime, adlar = 'fn', re.findall(r'`([A-Za-z0-9_]+)`', mm.group(1))
        if kelime:
            for ad in adlar:
                hedef.append((dosya, ad, kelime))
        else:
            kalan[w] += 1
    return hedef, kalan


def gecen(dosya, ad):
    desen = re.compile(r'\b' + ad + r'\b')
    return sum(len(desen.findall(oku(p))) for p in SRC)


def sil(dosya, ad, kelime):
    s = oku(dosya)
    i = s.find(kelime + ' ' + ad)
    if i < 0:
        return False
    bas = s.rfind('\n', 0, i) + 1
    while True:
        o = s.rfind('\n', 0, bas - 1)
        if o < 0:
            break
        satir = s[o + 1:bas].strip()
        if satir.startswith(('///', '//', '#[')):
            bas = o + 1
        else:
            break
    j = s.find('{', i)
    nokta = s.find(';', i)
    if j < 0 or (0 <= nokta < j):
        k = s.find('\n', nokta) + 1
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
        k = s.find('\n', j) + 1
    yaz(dosya, s[:bas] + s[k:])
    return True


for tur in range(4):
    hedef, kalan = tani()
    sayi = 0
    for dosya, ad, kelime in sorted(set(hedef)):
        if gecen(dosya, ad) != 1:
            continue
        if sil(dosya, ad, kelime):
            sayi += 1
            print('silindi:', dosya, ad)
    print('tur', tur, '->', sayi)
    if sayi == 0:
        print('kalan uyari cesidi:', len(kalan))
        for x, n in kalan.most_common(12):
            print('   ', n, x)
        break
