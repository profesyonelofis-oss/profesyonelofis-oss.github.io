"""O-5 koridoru (Kemalpaşa–Savaştepe) sıralama haritasını PNG olarak çizer.

Sayfa üreticisinin yazdığı bölge HTML'lerinden koordinatları okur, ölçüm
tablosundaki sıralarla eşler, otoyol hattını çizer ve `harita-siralama.png` üretir.
"""
import glob, io, math, os, re, urllib.request
from PIL import Image, ImageDraw, ImageFont

# Kapsam: yalnızca O-5 üzerindeki yedi ilçe (dosya adı -> sıramız, liderin yorumu).
OLCUM = {
    "izmir-kemalpasa": (0, 54),
    "manisa-turgutlu": (0, None),
    "manisa-saruhanli": (3, 23),
    "manisa-akhisar": (0, 42),
    "manisa-kirkagac": (0, 66),
    "manisa-soma": (0, 20),
    "balikesir-savastepe": (0, 108),
}
SIRA = ["izmir-kemalpasa", "manisa-turgutlu", "manisa-saruhanli", "manisa-akhisar",
        "manisa-kirkagac", "manisa-soma", "balikesir-savastepe"]
IL = {"izmir": "İzmir", "manisa": "Manisa", "balikesir": "Balıkesir"}
Z = 10
TILE = 256


def font(boyut, kalin=False):
    for yol in (r"C:\Windows\Fonts\segoeuib.ttf" if kalin else r"C:\Windows\Fonts\segoeui.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if kalin
                else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"):
        if os.path.exists(yol):
            return ImageFont.truetype(yol, boyut)
    return ImageFont.load_default()


def kar(cx, cy):
    x = int((cx + 180.0) / 360.0 * (2 ** Z))
    y = int((1.0 - math.log(math.tan(math.radians(cy)) + 1 / math.cos(math.radians(cy))) / math.pi)
            / 2.0 * (2 ** Z))
    return x, y


def konum(cx, cy):
    n = 2 ** Z * TILE
    x = (cx + 180.0) / 360.0 * n
    y = (1.0 - math.log(math.tan(math.radians(cy)) + 1 / math.cos(math.radians(cy))) / math.pi) / 2.0 * n
    return x, y


noktalar = {}
for yol in sorted(glob.glob("bolgesel/saruhanliotokurtarma/*.html")):
    ad = yol.replace("\\", "/").split("/")[-1].replace(".html", "")
    if ad not in OLCUM:
        continue
    s = open(yol, encoding="utf-8").read()
    m = re.search(r"marker=([\d.]+)%2C([\d.]+)", s)
    if not m:
        continue
    b = re.search(r"<title>([^<]+)</title>", s)
    etiket = b.group(1).split(" | ")[0] if b else ad
    for onek in ("Oto kurtarma ", "Çekici hizmeti ", "Çekici "):
        if etiket.startswith(onek):
            etiket = etiket[len(onek):]
            break
    sira, rakip = OLCUM[ad]
    noktalar[ad] = dict(ad=ad, ilce=etiket, il=IL.get(ad.split("-")[0], ""),
                        lat=float(m.group(1)), lon=float(m.group(2)), sira=sira, rakip=rakip)

liste = [noktalar[ad] for ad in SIRA if ad in noktalar]
enlemler = [n["lat"] for n in liste]
boylamlar = [n["lon"] for n in liste]
ust, alt = max(enlemler) + 0.20, min(enlemler) - 0.20
sol, sag = min(boylamlar) - 0.30, max(boylamlar) + 0.34

x0, y0 = kar(sol, ust)
x1, y1 = kar(sag, alt)
gen, yuk = (x1 - x0 + 1) * TILE, (y1 - y0 + 1) * TILE
harita = Image.new("RGB", (gen, yuk), "#e8ede6")

for tx in range(x0, x1 + 1):
    for ty in range(y0, y1 + 1):
        url = f"https://tile.openstreetmap.org/{Z}/{tx}/{ty}.png"
        try:
            with urllib.request.urlopen(
                    urllib.request.Request(url, headers={"User-Agent": "siralama-haritasi/1.0"}),
                    timeout=15) as y:
                kare = Image.open(io.BytesIO(y.read())).convert("RGB")
        except Exception:
            continue
        harita.paste(kare, ((tx - x0) * TILE, (ty - y0) * TILE))

kx0, ky0 = konum(sol, ust)
ciz = ImageDraw.Draw(harita, "RGBA")
kalin = font(32, True)
orta = font(20, True)
kucuk = font(17)
minik = font(13)


def yeri(n):
    return konum(n["lon"], n["lat"])[0] - kx0, konum(n["lon"], n["lat"])[1] - ky0


# Otoyol hattı
hat = [yeri(n) for n in liste]
if len(hat) > 1:
    ciz.line(hat, fill=(18, 82, 63, 150), width=9, joint="curve")
    ciz.line(hat, fill=(255, 255, 255, 220), width=4, joint="curve")


def renk(n):
    if n["sira"] is None:
        return (201, 209, 203, 235)
    if 1 <= n["sira"] <= 3:
        return (18, 82, 63, 250)
    if 4 <= n["sira"] <= 10:
        return (184, 134, 11, 240)
    return (154, 166, 160, 240)


def yaricap(n):
    if not n["rakip"]:
        return 13
    return int(min(30, 12 + math.sqrt(n["rakip"]) * 0.95))


for n in sorted(liste, key=lambda k: k["rakip"] or 0, reverse=True):
    px, py = yeri(n)
    r = yaricap(n)
    ciz.ellipse([px - r - 3, py - r - 3, px + r + 3, py + r + 3], fill=(255, 255, 255, 240))
    ciz.ellipse([px - r, py - r, px + r, py + r], fill=renk(n), outline=(255, 255, 255, 255), width=3)

for n in liste:
    px, py = yeri(n)
    r = yaricap(n)
    if n["sira"] and n["sira"] >= 1:
        ciz.ellipse([px - r - 13, py - r - 13, px + r + 13, py + r + 13],
                    outline=(18, 82, 63, 200), width=4)
        yazi = f'{n["ilce"]} · {n["sira"]}. sıra'
        renk_yazi = (18, 82, 63, 255)
    else:
        yazi = f'{n["ilce"]} · yok'
        renk_yazi = (70, 80, 74, 255)
    g = ciz.textbbox((0, 0), yazi, font=kucuk)
    w, hh = g[2] - g[0], g[3] - g[1]
    bx = px + r + 12
    by = py - hh / 2 - 8
    ciz.rounded_rectangle([bx - 8, by - 5, bx + w + 8, by + hh + 7], 7,
                          fill=(255, 255, 255, 238), outline=(210, 220, 213, 255), width=1)
    ciz.text((bx, by - 1), yazi, font=kucuk, fill=renk_yazi)

# Panel
px0, py0 = 28, 28
ciz.rounded_rectangle([px0, py0, px0 + 372, py0 + 286], 16, fill=(255, 255, 255, 250),
                      outline=(221, 229, 223, 255), width=2)
ciz.text((px0 + 22, py0 + 18), "O-5 koridoru · sıralama", font=kalin, fill=(17, 24, 20, 255))
ciz.text((px0 + 22, py0 + 60), "İzmir Kemalpaşa → Balıkesir Savaştepe", font=minik, fill=(91, 102, 95, 255))
ciz.text((px0 + 22, py0 + 78), "Google Haritalar · “çekici <ilçe>” · 10 Kasım 2026", font=minik,
         fill=(91, 102, 95, 255))
for i, (sayi, ad) in enumerate([("7", "bölge"), ("1", "görünür"), ("3.", "en iyi sıra")]):
    bx = px0 + 22 + i * 112
    ciz.rounded_rectangle([bx, py0 + 104, bx + 104, py0 + 166], 10, fill=(244, 247, 243, 255))
    ciz.text((bx + 12, py0 + 112), sayi, font=orta, fill=(17, 24, 20, 255))
    ciz.text((bx + 12, py0 + 142), ad, font=minik, fill=(91, 102, 95, 255))

lej = [((18, 82, 63), "İlk 3 — görünüyoruz"), ((184, 134, 11), "4–10 arası"),
       ((154, 166, 160), "İlk 6’da yok")]
for i, (c, ad) in enumerate(lej):
    ly = py0 + 188 + i * 26
    ciz.ellipse([px0 + 24, ly, px0 + 40, ly + 16], fill=c + (255,))
    ciz.text((px0 + 50, ly - 3), ad, font=minik, fill=(17, 24, 20, 255))
ciz.text((px0 + 22, py0 + 268), "Daire boyutu = ilçedeki liderin yorum sayısı", font=minik,
         fill=(107, 118, 111, 255))

at = "© OpenStreetMap katkıcıları · konum belirtilmeden arama"
g = ciz.textbbox((0, 0), at, font=minik)
ciz.rectangle([gen - (g[2] - g[0]) - 26, yuk - 30, gen - 6, yuk - 4], fill=(255, 255, 255, 220))
ciz.text((gen - (g[2] - g[0]) - 16, yuk - 27), at, font=minik, fill=(70, 80, 74, 255))

if harita.width > 1200:
    harita = harita.resize((1200, int(harita.height * 1200 / harita.width)), Image.LANCZOS)
harita.save("harita-siralama.png", optimize=True)
print("yazildi: harita-siralama.png", harita.size, "| bölge:", len(liste))
