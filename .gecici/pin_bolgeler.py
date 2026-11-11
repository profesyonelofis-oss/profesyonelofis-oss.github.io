from __future__ import annotations
"""30 bölgenin tek tek durumunu PNG tabloya çizer."""
import os
from PIL import Image, ImageDraw, ImageFont

KLASOR = "bolgesel/saruhanliotokurtarma"
W, PAD = 1040, 40
CREAM, DIM, FAINT = (244, 241, 232), (152, 173, 160), (118, 141, 128)
LIME, AMBER = (200, 241, 105), (255, 179, 71)
CARD, LINE, HDRBG = (9, 27, 20), (255, 255, 255, 28), (255, 255, 255, 10)
F, FB = "C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/segoeuib.ttf"

# il, ilce, dosya, dalga
BOLGELER = [
    ("İzmir", "Aliağa", "izmir-aliaga", 1),
    ("İzmir", "Menemen", "izmir-menemen", 1),
    ("İzmir", "Çiğli", "izmir-cigli", 1),
    ("İzmir", "Bornova", "izmir-bornova", 1),
    ("İzmir", "Gaziemir", "izmir-gaziemir", 1),
    ("İzmir", "Torbalı", "izmir-torbali", 1),
    ("Manisa", "Soma", "manisa-soma", 1),
    ("Manisa", "Kırkağaç", "manisa-kirkagac", 1),
    ("Manisa", "Akhisar", "manisa-akhisar", 1),
    ("Manisa", "Saruhanlı", "manisa-saruhanli", 1),
    ("Manisa", "Şehzadeler", "manisa-sehzadeler", 1),
    ("Manisa", "Yunusemre", "manisa-yunusemre", 1),
    ("Balıkesir", "Savaştepe", "balikesir-savastepe", 1),
    ("Otoyol", "O-5 Otoyolu", "otoyol-o5otoyolu", 1),
    ("Otoyol", "İzmir–İstanbul Otoyolu", "otoyol-izmiristanbulotoyolu", 1),
    ("İstanbul", "Tuzla", "istanbul-tuzla", 2),
    ("İstanbul", "Pendik", "istanbul-pendik", 2),
    ("İstanbul", "Kartal", "istanbul-kartal", 2),
    ("Kocaeli", "Gebze", "kocaeli-gebze", 2),
    ("Kocaeli", "Dilovası", "kocaeli-dilovasi", 2),
    ("Kocaeli", "Çayırova", "kocaeli-cayirova", 2),
    ("Yalova", "Altınova", "yalova-altinova", 2),
    ("Yalova", "Çiftlikköy", "yalova-ciftlikkoy", 2),
    ("Yalova", "Çınarcık", "yalova-cinarcik", 2),
    ("Bursa", "Orhangazi", "bursa-orhangazi", 2),
    ("Bursa", "Gemlik", "bursa-gemlik", 2),
    ("Bursa", "Nilüfer", "bursa-nilufer", 2),
    ("Bursa", "Karacabey", "bursa-karacabey", 2),
    ("Balıkesir", "Merkez", "balikesir-merkez", 2),
    ("Balıkesir", "Susurluk", "balikesir-susurluk", 2),
]

eksik = []
boyutlar = {}
for il, ilce, slug, dalga in BOLGELER:
    yol = os.path.join(KLASOR, slug + ".html")
    if os.path.exists(yol):
        boyutlar[slug] = max(1, round(os.path.getsize(yol) / 1024))
    else:
        eksik.append(slug)
        boyutlar[slug] = 0
if eksik:
    print("EKSIK SAYFA:", eksik)
print("sayfa:", len(BOLGELER) - len(eksik), "/", len(BOLGELER), "| toplam KB:", sum(boyutlar.values()))

# küçük harf: başlıktaki kelime
def kucuk(s: str) -> str:
    return s.replace("I", "ı").replace("İ", "i").lower()


def font(size: float, bold: bool = False):
    return ImageFont.truetype(FB if bold else F, int(size))


img = Image.new("RGB", (W, 2200), (7, 17, 13))
d = ImageDraw.Draw(img, "RGBA")
for yy in range(2200):
    t = min(1.0, yy / 640)
    d.line([(0, yy), (W, yy)], fill=(round(13 + (7 - 13) * t), round(58 + (17 - 58) * t), round(41 + (13 - 41) * t)))

y = 32
d.rounded_rectangle([PAD, y, PAD + 52, y + 52], 14, fill=(10, 51, 38), outline=(200, 241, 105, 90), width=1)
d.text((PAD + 15, y + 10), "S", font=font(25, True), fill=LIME)
d.text((PAD + 68, y), "30 bölge · tek tek durum", font=font(31, True), fill=CREAM)
d.text((PAD + 68, y + 42), "Saruhanlı Oto Kurtarma  ·  O-5 koridoru  ·  sayfa + pin + hedef kelime + öncelik dalgası",
       font=font(12.5), fill=DIM)
et = "10 KASIM 2026"
tw = d.textlength(et, font=font(11, True))
d.rounded_rectangle([W - PAD - tw - 32, y + 4, W - PAD, y + 29], 13, fill=(255, 255, 255, 12), outline=LINE, width=1)
d.text((W - PAD - tw - 16, y + 9), et, font=font(11, True), fill=(219, 231, 223))
y += 66
d.line([(PAD, y), (W - PAD, y)], fill=LINE)
y += 16

# KPI
kpis = [("30", "BÖLGE", LIME), ("15", "1. DALGA", LIME), ("15", "2. DALGA", (150, 190, 235)),
        ("8", "ORTAK KELİME", LIME), (f"{len(BOLGELER) - len(eksik)}/30", "SAYFA HAZIR", LIME if not eksik else AMBER)]
kw = (W - 2 * PAD - 4 * 9) / 5
for i, (num, lab, col) in enumerate(kpis):
    x = PAD + i * (kw + 9)
    d.rounded_rectangle([x, y, x + kw, y + 76], 12, fill=(9, 27, 20, 235), outline=LINE, width=1)
    d.text((x + 15, y + 9), num, font=font(28, True), fill=col)
    d.text((x + 15, y + 52), lab, font=font(9, True), fill=FAINT)
y += 98

sutunlar = ["#", "BÖLGE", "SAYFA", "PİN", "HEDEF KELİME (SAYFA)", "HARİTA", "DALGA"]
g = [40, 215, 92, 62, 312, 70, 132]
satir_h = 33
x0, x1 = PAD, W - PAD
tablo_h = 32 + satir_h * len(BOLGELER)
d.rounded_rectangle([x0, y, x1, y + tablo_h], 13, fill=CARD, outline=LINE, width=1)
d.rectangle([x0 + 1, y + 1, x1 - 1, y + 32], fill=HDRBG)
xs, cx = [], x0 + 16
for wgen in g:
    xs.append(cx)
    cx += wgen
for i, s in enumerate(sutunlar):
    d.text((xs[i], y + 10), s, font=font(9, True), fill=FAINT)
d.line([(x0 + 1, y + 32), (x1 - 1, y + 32)], fill=LINE)

ry = y + 32
for ri, (il, ilce, slug, dalga) in enumerate(BOLGELER):
    if ri:
        d.line([(x0 + 1, ry), (x1 - 1, ry)], fill=(255, 255, 255, 12))
    if ri % 2 == 0:
        d.rectangle([x0 + 1, ry + 1, x1 - 1, ry + satir_h - 1], fill=(255, 255, 255, 5))
    d.text((xs[0], ry + 10), str(ri + 1).zfill(2), font=font(9.5, True), fill=FAINT)
    d.text((xs[1], ry + 9), ilce, font=font(12, True), fill=CREAM)
    d.text((xs[1] + d.textlength(ilce, font=font(12, True)) + 7, ry + 11), "· " + il, font=font(10), fill=FAINT)

    def nokta(x: int, renk) -> None:
        d.ellipse([x, ry + 13, x + 6, ry + 19], fill=renk)

    # sayfa
    boy = boyutlar[slug]
    nokta(xs[2], LIME if boy else AMBER)
    d.text((xs[2] + 14, ry + 10), f"{boy} KB" if boy else "yok",
           font=font(10.5), fill=(219, 231, 223) if boy else AMBER)

    # pin
    nokta(xs[3], LIME)
    d.text((xs[3] + 14, ry + 10), "var", font=font(10.5), fill=(200, 214, 206))

    # hedef kelime
    d.text((xs[4], ry + 10), "oto kurtarma " + kucuk(ilce), font=font(11), fill=(219, 231, 223))

    # harita
    nokta(xs[5], LIME)
    d.text((xs[5] + 14, ry + 10), "link", font=font(10), fill=(160, 182, 169))

    # dalga
    if dalga == 1:
        metin, col = "1. DALGA", LIME
    else:
        metin, col = "2. DALGA", (150, 190, 235)
    tw = d.textlength(metin, font=font(9.5, True))
    d.rounded_rectangle([xs[6] - 2, ry + 7, xs[6] + tw + 20, ry + 26], 10,
                        fill=(col[0], col[1], col[2], 26), outline=(col[0], col[1], col[2], 70), width=1)
    d.text((xs[6] + 8, ry + 11), metin, font=font(9.5, True), fill=col)
    ry += satir_h
y += tablo_h + 16

for satir, renk in [
    ("Hedef kelime = sayfanın başlık kelimesi (oto kurtarma + ilçe). 30 sayfanın hepsi ayrıca bölgesiz 8 ortak kelimeye hizmet eder:", CREAM),
    ("çekici · oto kurtarma · yol yardım 7/24 · en yakın çekici · benzin destek · mazot takviye · akü takviye · akücü", LIME),
    ("Google sırası: ölçülmedi — site yayında değil (alan adı, profil doğrulama, Search Console bekliyor).", DIM),
    ("1. dalga: ilk yayına alınacak ve yorum toplanacak 15 bölge · 2. dalga: segment oturduktan sonra (Bursa, Yalova, Kocaeli, İstanbul).", DIM),
]:
    d.text((PAD, y), satir, font=font(11.5, True if renk is CREAM else False), fill=renk)
    y += 18

y += 8
d.line([(PAD, y), (W - PAD, y)], fill=LINE)
y += 12
d.text((PAD, y), f"Kaynak: {KLASOR}/ · 30 html + sitemap.xml + robots.txt + plan.md + siralama-plani.md + gonderiler.md + profil.md  ·  sayfa toplamı ≈ {sum(boyutlar.values())} KB",
       font=font(10), fill=FAINT)
y += 20

img.crop((0, 0, W, y)).save("pin-bolgeler.png")
print("yazildi: pin-bolgeler.png", (W, y))
