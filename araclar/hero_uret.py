"""O-5 çekici sitesi için 3B hero videoları üretir.

Kareler PIL ile 3B projeksiyon kullanılarak çizilir, ffmpeg ile H.264 mp4'e
çevrilir. Dış kaynak/stok görüntü yok; her sahne sıfırdan üretilir.

Kullanım:
    python araclar/hero_uret.py            # dört videoyu da üret
    python araclar/hero_uret.py --tek      # her sahneden bir kare (kontrol)
"""
import math
import os
import random
import shutil
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

W, H = 960, 540
FPS = 24
SURE = 6.0
N = int(FPS * SURE)
KOK = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CIK = os.path.join(KOK, "site", "video", "hero")
GEC = os.path.join(KOK, "_gecici", "hero_kareler")
FOV = 34.0
F = W / (2 * math.tan(math.radians(FOV / 2)))


# ---------------------------------------------------------------- vektörler
def vadd(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def vsub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def vmul(a, s):
    return (a[0] * s, a[1] * s, a[2] * s)


def vdot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def vcross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def vnorm(a):
    m = math.sqrt(vdot(a, a)) or 1.0
    return (a[0] / m, a[1] / m, a[2] / m)


def karisim(a, b, t):
    t = max(0.0, min(1.0, t))
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


# ------------------------------------------------------------------ kamera
class Kamera:
    def __init__(self, konum, hedef, aci=0.0):
        self.konum = konum
        d = vnorm(vsub(hedef, konum))
        r = vnorm(vcross(d, (0, 1, 0)))
        u = vcross(r, d)
        self.d, self.r, self.u = d, r, u
        if aci:
            ca, sa = math.cos(aci), math.sin(aci)
            self.r = vadd(vmul(self.r, ca), vmul(self.u, sa))
            self.u = vadd(vmul(self.u, ca), vmul(self.r, -sa))

    def proj(self, p):
        v = vsub(p, self.konum)
        z = vdot(v, self.d)
        if z < 0.25:
            return None
        x = vdot(v, self.r)
        y = vdot(v, self.u)
        return (W / 2 + F * x / z, H / 2 - F * y / z, z)


# ------------------------------------------------------------------- yüzey
def kutu(merkez, yari, aci=0.0):
    """Eksen hizalı (istenirse z ekseninde döndürülmüş) kutunun 6 yüzü."""
    cx, cy, cz = merkez
    hx, hy, hz = yari
    ca, sa = math.cos(aci), math.sin(aci)
    pts = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            for sz in (-1, 1):
                x, y = sx * hx, sy * hy
                pts.append((cx + x * ca - y * sa, cy + x * sa + y * ca, cz + sz * hz))
    i = lambda a, b, c: a * 4 + b * 2 + c
    yuzler = [
        ((0, 0, 0), (0, 0, 1), (0, 1, 1), (0, 1, 0)),
        ((1, 0, 0), (1, 1, 0), (1, 1, 1), (1, 0, 1)),
        ((0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1)),
        ((0, 1, 0), (0, 1, 1), (1, 1, 1), (1, 1, 0)),
        ((0, 0, 0), (0, 1, 0), (1, 1, 0), (1, 0, 0)),
        ((0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)),
    ]
    return [[pts[i(*y)] for y in yuz] for yuz in yuzler]


def yuz_merkez(yuz):
    n = len(yuz)
    return (
        sum(p[0] for p in yuz) / n,
        sum(p[1] for p in yuz) / n,
        sum(p[2] for p in yuz) / n,
    )


def yuz_normal(yuz):
    a, b, c = yuz[0], yuz[1], yuz[2]
    return vnorm(vcross(vsub(b, a), vsub(c, a)))


def yuz_ciz(dr, kam, yuz, renk, isik, sis, sis_renk, alfa=255):
    n = yuz_normal(yuz)
    merkez = yuz_merkez(yuz)
    if vdot(n, vsub(merkez, kam.konum)) > 0:  # arkaya bakan yüz
        return
    lam = max(0.0, vdot(n, isik))
    taban = 0.34 + 0.66 * lam
    olcek = max(0.35, min(1.25, taban))
    c = tuple(min(255, int(v * olcek)) for v in renk)
    z = vdot(vsub(merkez, kam.konum), kam.d)
    c = karisim(c, sis_renk, min(1.0, sis * max(0.0, z - 6) / 90.0))
    noktalar = []
    for p in yuz:
        q = kam.proj(p)
        if q is None:
            return
        noktalar.append((q[0], q[1]))
    dr.polygon(noktalar, fill=c + (alfa,) if len(c) == 3 else c)


def zemin_ciz(dr, kam, renk, sis_renk, sis, genislik=90.0):
    """Yolu ve çevresini 2 m'lik şeritler halinde çizer (sis derinlikle artar)."""
    for i in range(-45, 46, 2):
        x0, x1 = i, i + 2
        quad = [
            (x0, 0.0, -genislik),
            (x0, 0.0, genislik),
            (x1, 0.0, genislik),
            (x1, 0.0, -genislik),
        ]
        z = abs(i)
        t = min(1.0, sis * max(0.0, z - 10) / 110.0)
        dr.polygon(
            [kam.proj(p)[:2] for p in quad if kam.proj(p)],
            fill=karisim(renk, sis_renk, t),
        )


# ------------------------------------------------------------------ ışıklar
def parlama(layer, x, y, r, renk, guc=200):
    g = Image.new("RGBA", (int(r * 2), int(r * 2)), (0, 0, 0, 0))
    d = ImageDraw.Draw(g)
    for k in range(8, 0, -1):
        rr = r * k / 8
        a = int(guc * (1 - k / 9) ** 1.6)
        d.ellipse((r - rr, r - rr, r + rr, r + rr), fill=renk + (a,))
    layer.alpha_composite(g, (int(x - r), int(y - r)))


def gokyuzu(ust, alt):
    im = Image.new("RGB", (W, H))
    d = ImageDraw.Draw(im)
    for y in range(H):
        d.line([(0, y), (W, y)], fill=karisim(ust, alt, y / H))
    return im


def vinyet(im, guc=90):
    v = Image.new("L", (W, H), 0)
    d = ImageDraw.Draw(v)
    d.ellipse((-W * 0.35, -H * 0.5, W * 1.35, H * 1.5), fill=255)
    v = v.filter(ImageFilter.GaussianBlur(110))
    karart = Image.new("RGB", (W, H), (0, 0, 0))
    return Image.composite(im, Image.blend(im, karart, guc / 255), v)


# ------------------------------------------------------------------- çekici
def cekici(dr, kam, isik, sis, sis_renk, govde=(24, 60, 46), aksan=(217, 255, 87)):
    koyu = (16, 20, 18)
    cam = (46, 60, 58)
    lastik = (22, 24, 26)
    parcalar = []
    # şasi + platform
    parcalar.append((kutu((0, 0.62, 0), (3.6, 0.16, 1.15)), koyu))
    parcalar.append((kutu((0, 0.92, 0), (3.5, 0.14, 1.2)), aksan))
    parcalar.append((kutu((0, 1.04, 0), (3.35, 0.04, 1.1)), (58, 66, 63)))
    # kabin
    parcalar.append((kutu((2.35, 1.72, 0), (0.95, 0.72, 1.12)), govde))
    parcalar.append((kutu((3.22, 1.86, 0), (0.06, 0.36, 0.95)), cam))
    parcalar.append((kutu((2.35, 1.2, 1.13), (0.7, 0.34, 0.02)), cam))
    parcalar.append((kutu((2.35, 1.2, -1.13), (0.7, 0.34, 0.02)), cam))
    # tepe lambası
    parcalar.append((kutu((2.1, 2.5, 0), (0.42, 0.1, 0.5)), (255, 196, 62)))
    # tekerler
    for x in (-2.35, -1.2, 2.2):
        for z in (-1.16, 1.16):
            parcalar.append((kutu((x, 0.42, z), (0.43, 0.42, 0.16)), lastik))
    # platformdaki araç
    parcalar.append((kutu((-0.15, 1.42, 0), (1.95, 0.34, 0.84)), (226, 228, 224)))
    parcalar.append((kutu((-0.35, 1.92, 0), (0.95, 0.3, 0.74)), (206, 210, 208)))
    parcalar.append((kutu((-0.35, 1.93, 0), (0.86, 0.26, 0.76)), (38, 44, 44)))
    # bağlama kayışları
    for x in (-1.1, 0.7):
        parcalar.append((kutu((x, 1.55, 0.86), (0.05, 0.55, 0.03)), (236, 168, 42)))
    # kurtarma vinci (döndürülmüş kol)
    parcalar.append((kutu((-2.6, 1.55, 0), (0.16, 1.1, 0.2), aci=math.radians(-22)), (176, 44, 38)))
    parcalar.append((kutu((-3.05, 2.35, 0), (0.9, 0.08, 0.13), aci=math.radians(28)), (176, 44, 38)))
    # çizim sırası: derinliğe göre uzak -> yakın
    kareler = []
    for yuzler, renk in parcalar:
        for y in yuzler:
            m = yuz_merkez(y)
            kareler.append((vdot(vsub(m, kam.konum), kam.d), y, renk))
    for _, y, renk in sorted(kareler, key=lambda q: -q[0]):
        yuz_ciz(dr, kam, y, renk, isik, sis, sis_renk)


def bariyer(dr, kam, isik, sis, sis_renk, renk=(120, 132, 126)):
    for z in (-5.0, 5.0):
        for x in range(-18, 19, 3):
            yuz_ciz(dr, kam, kutu((x, 0.55, z), (0.09, 0.55, 0.09))[0], renk, isik, sis, sis_renk)
        yuz_ciz(dr, kam, kutu((0, 0.95, z), (18.0, 0.12, 0.07))[0], (150, 162, 156), isik, sis, sis_renk)
        yuz_ciz(dr, kam, kutu((0, 0.95, z), (18.0, 0.12, 0.07))[1], (150, 162, 156), isik, sis, sis_renk)


def serit(dr, kam, renk, kayma, sis, sis_renk):
    """Yol kenar ve orta çizgileri (kayma ile akar)."""
    adim = 3.2
    for z in (-3.5, 3.5, 0.0):
        for i in range(-14, 15):
            x0 = i * adim + (kayma % adim)
            quad = [(x0, 0.012, z - 0.07), (x0, 0.012, z + 0.07),
                    (x0 + 1.7, 0.012, z + 0.07), (x0 + 1.7, 0.012, z - 0.07)]
            nk = [kam.proj(p) for p in quad]
            if all(nk):
                dr.polygon([p[:2] for p in nk], fill=renk)


# ----------------------------------------------------------------- sahneler
def sahne_orbit(t, mod):
    aksam = mod in ("gece", "yagmur")
    if mod == "gece":
        sis_renk = (12, 18, 24)
        gok = gokyuzu((6, 10, 16), (24, 32, 44))
        isik = vnorm((-0.4, -0.55, 0.6))
        zemin = (20, 24, 28)
    elif mod == "yagmur":
        sis_renk = (58, 66, 70)
        gok = gokyuzu((52, 60, 66), (96, 104, 108))
        isik = vnorm((-0.5, -0.6, 0.4))
        zemin = (40, 46, 50)
    else:
        sis_renk = (196, 208, 200)
        gok = gokyuzu((126, 168, 168), (222, 226, 210))
        isik = vnorm((-0.55, -0.6, 0.45))
        zemin = (66, 72, 70)

    im = gok.convert("RGBA")
    dr = ImageDraw.Draw(im)

    # ufukta tepe siluetleri (kamera açısıyla kayar)
    theta = 2 * math.pi * t
    for k, (yuk, renk, hiz) in enumerate(
        ((46, karisim(sis_renk, (60, 74, 70), 0.35), 26), (30, karisim(sis_renk, (46, 58, 54), 0.55), 40))
    ):
        noktalar = [(0, H / 2)]
        for x in range(0, W + 20, 20):
            yy = H / 2 - yuk * (0.55 + 0.45 * math.sin((x + theta * hiz * 6 + k * 400) / 150.0))
            noktalar.append((x, yy))
        noktalar.append((W, H / 2))
        dr.polygon(noktalar, fill=renk)

    zemin_ciz(dr, Kamera((0, 1.4, 9), (0, 1.2, 0)), zemin, sis_renk, 1.0)

    kam = Kamera((11.5 * math.sin(theta), 3.1 + 0.5 * math.cos(theta), 11.5 * math.cos(theta)),
                 (0, 1.35, 0))
    serit(dr, kam, karisim((235, 238, 232), sis_renk, 0.25 if aksam else 0.05),
          t * 6.4, 0.9, sis_renk)
    bariyer(dr, kam, isik, 0.75, sis_renk,
            renk=(96, 104, 100) if aksam else (128, 140, 134))
    cekici(dr, kam, isik, 0.85, sis_renk,
           govde=(20, 52, 40) if not aksam else (18, 40, 33))

    # ışıklar
    isik_kat = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    far = kam.proj((3.3, 0.9, 1.05)) if kam else None
    if aksam:
        for zz in (-1.05, 1.05):
            q = kam.proj((3.34, 0.86, zz))
            if q and 0 < q[0] < W and 0 < q[1] < H:
                parlama(isik_kat, q[0], q[1], 90 if mod == "gece" else 60, (255, 240, 190), 210)
        q = kam.proj((2.1, 2.55, 0))
        if q and 0 < q[0] < W and 0 < q[1] < H:
            parlama(isik_kat, q[0], q[1], 52, (255, 196, 80), 230)
    elif far and 0 < far[0] < W:
        parlama(isik_kat, far[0], far[1], 70, (255, 250, 220), 90)

    if mod == "yagmur":
        rnd = random.Random(7)
        d2 = ImageDraw.Draw(isik_kat)
        for _ in range(320):
            x = rnd.uniform(0, W)
            y = (rnd.uniform(0, H) + t * H * 2.6) % (H * 1.1)
            d2.line((x, y, x - 3, y + 16), fill=(210, 226, 232, 90), width=1)

    isik_kat = isik_kat.filter(ImageFilter.GaussianBlur(0.6))
    im = Image.alpha_composite(im, isik_kat)
    return vinyet(im.convert("RGB"), 110 if aksam else 70)


def sahne_koridor(t):
    im = gokyuzu((8, 12, 14), (16, 26, 24)).convert("RGBA")
    dr = ImageDraw.Draw(im)
    sis_renk = (12, 18, 16)

    # O-5 hattı: hafif kavisli şerit
    noktalar = []
    for i in range(0, 130):
        x = i * 2.2
        z = 22 * math.sin(i * 0.055)
        noktalar.append((x, 0.0, z))
    km = [0, 30, 50, 75, 90, 112, 127]
    adlar = ["K22 Kemalpaşa", "K21 Turgutlu", "K19 Saruhanlı", "K18 Akhisar",
             "K17 Kırkağaç", "K16 Soma", "K15 Savaştepe"]

    ilerleme = 0.5 - 0.5 * math.cos(2 * math.pi * t)  # ileri-geri, dikişsiz
    kam_x = 20 + ilerleme * 180
    kam = Kamera((kam_x, 46, -46), (kam_x + 26, 0, 6))

    # yol şeridi
    for i in range(len(noktalar) - 1):
        a, b = noktalar[i], noktalar[i + 1]
        yon = vnorm((b[0] - a[0], 0, b[2] - a[2]))
        yan = vnorm(vcross(yon, (0, 1, 0)))
        w = 7.0
        quad = [
            vadd(vadd(a, vmul(yan, -w)), (0, 0.1, 0)),
            vadd(vadd(a, vmul(yan, w)), (0, 0.1, 0)),
            vadd(vadd(b, vmul(yan, w)), (0, 0.1, 0)),
            vadd(vadd(b, vmul(yan, -w)), (0, 0.1, 0)),
        ]
        pj = [kam.proj(p) for p in quad]
        if all(pj):
            olcek = min(1.0, 0.30 + 1.0 / (1 + pj[0][2] / 60))
            dr.polygon([p[:2] for p in pj], fill=karisim((26, 34, 30), sis_renk, 1 - olcek))
        # orta çizgi
        for k in (0.25, 0.55, 0.85):
            c0 = vadd(a, vmul(vsub(b, a), k))
            c1 = vadd(a, vmul(vsub(b, a), k + 0.14))
            quad = [vadd(vadd(c0, vmul(yan, -0.16)), (0, 0.14, 0)),
                    vadd(vadd(c0, vmul(yan, 0.16)), (0, 0.14, 0)),
                    vadd(vadd(c1, vmul(yan, 0.16)), (0, 0.14, 0)),
                    vadd(vadd(c1, vmul(yan, -0.16)), (0, 0.14, 0))]
            pj = [kam.proj(p) for p in quad]
            if all(pj):
                dr.polygon([p[:2] for p in pj], fill=(190, 205, 196))

    # kavşak işaretleri + etiketler
    try:
        font = ImageFont.truetype("C:/Windows/Fonts/arialbd.ttf", 26)
        kucuk = ImageFont.truetype("C:/Windows/Fonts/arial.ttf", 17)
    except OSError:
        font = kucuk = ImageFont.load_default()
    for i, (k, ad) in enumerate(zip(km, adlar)):
        idx = int(min(len(noktalar) - 2, k / 2.2 * 1.0))
        p = noktalar[idx]
        yan = vnorm(vcross((1, 0, 0), (0, 1, 0)))
        taban = vadd(p, (0, 0.15, 0))
        pj_t = kam.proj(taban)
        pj_u = kam.proj(vadd(taban, (0, 6.5, 0)))
        if pj_t and pj_u:
            dr.line([pj_t[:2], pj_u[:2]], fill=(217, 255, 87), width=2)
            kure = kam.proj(vadd(taban, (0, 6.5, 0)))
            if kure:
                r = max(3, 190 / kure[2])
                dr.ellipse((kure[0] - r, kure[1] - r, kure[0] + r, kure[1] + r),
                           fill=(217, 255, 87))
                # etiket
                olcek = max(0.55, min(1.5, 34 / kure[2]))
                f2 = font if olcek > 0.95 else kucuk
                metin = f"{ad}"
                alt = f"{k} km"
                kw = dr.textlength(metin, font=f2)
                x0 = kure[0] - 8
                y0 = kure[1] - 42 * olcek
                dr.rounded_rectangle(
                    (x0 - 4, y0 - 4, x0 + kw + 10, y0 + 156 * olcek), radius=6,
                    fill=(9, 13, 11, 210), outline=(60, 76, 66))
                dr.text((x0, y0), metin, font=f2, fill=(244, 241, 232))
                dr.text((x0, y0 + 32 * olcek), alt, font=kucuk, fill=(217, 255, 87))

    # başlık
    try:
        bas = ImageFont.truetype("C:/Windows/Fonts/arialbd.ttf", 30)
    except OSError:
        bas = font
    dr.text((48, 44), "O-5 · KEMALPAŞA → SAVAŞTEPE", font=bas, fill=(217, 255, 87))
    dr.text((48, 86), "7 kavşak · 127 km · 7/24", font=kucuk, fill=(196, 208, 200))
    return vinyet(im.convert("RGB"), 120)


# ------------------------------------------------------------------ kodlama
def kodla(ad, kare_uret):
    klasor = os.path.join(GEC, ad)
    if os.path.isdir(klasor):
        shutil.rmtree(klasor)
    os.makedirs(klasor, exist_ok=True)
    for i in range(N):
        kare_uret(i / N).save(os.path.join(klasor, f"{i:04d}.jpg"), quality=88)
    os.makedirs(CIK, exist_ok=True)
    mp4 = os.path.join(CIK, f"{ad}.mp4")
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-framerate", str(FPS),
         "-i", os.path.join(klasor, "%04d.jpg"),
         "-c:v", "libx264", "-preset", "slow", "-crf", "30",
         "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an", mp4],
        check=True,
    )
    kare_uret(0).convert("RGB").resize((960, 540)).save(
        os.path.join(CIK, f"{ad}.jpg"), quality=82, optimize=True, progressive=True)
    boy = os.path.getsize(mp4) / 1024
    print(f"{ad}.mp4  {boy:.0f} KB")


def main():
    tek = "--tek" in sys.argv
    if tek:
        os.makedirs(os.path.join(GEC, "kontrol"), exist_ok=True)
        for mod in ("gunduz", "gece", "yagmur"):
            sahne_orbit(0.18, mod).save(os.path.join(GEC, "kontrol", f"{mod}.jpg"), quality=90)
        sahne_koridor(0.35).save(os.path.join(GEC, "kontrol", "koridor.jpg"), quality=90)
        print("kontrol kareleri:", os.path.join(GEC, "kontrol"))
        return
    kodla("koridor", lambda t: sahne_koridor(t))
    for mod in ("gunduz", "gece", "yagmur"):
        kodla(mod, lambda t, m=mod: sahne_orbit(t, m))


if __name__ == "__main__":
    main()
