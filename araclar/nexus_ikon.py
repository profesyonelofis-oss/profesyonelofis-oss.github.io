"""Nexus Enterprise uygulama ikonu.

Konsept: gece safiri (#0b131f) zemin üzerinde kehribar (#e5a93b) "N" monogramı.
Renkler uygulamanın kendi paletinden alınır; ince çizgi/desen yoktur ki ikon
16×16'da da okunur kalsın. Çıktı:
  varliklar/nexus.ico        -> .exe dosya ikonu (16/24/32/48/64/128/256)
  varliklar/nexus_256.png    -> pencere ikonu (eframe ViewportBuilder)
"""

from pathlib import Path

from PIL import Image, ImageDraw

KOK = Path(__file__).resolve().parent.parent
VARLIK = KOK / "varliklar"

GECE_UST = (16, 27, 43)
GECE_ALT = (7, 12, 20)
KEHRIBAR_UST = (242, 196, 106)
KEHRIBAR_ALT = (208, 146, 42)
KENAR = (229, 169, 59)

BOY = 1024


def dikey_gecis(ust, alt, genislik, yukseklik):
    """İki renk arasında dikey degrade üretir (yumuşak, bant izi bırakmadan)."""
    katman = Image.new("RGB", (1, yukseklik))
    for y in range(yukseklik):
        oran = y / max(1, yukseklik - 1)
        katman.putpixel(
            (0, y),
            tuple(round(ust[i] + (alt[i] - ust[i]) * oran) for i in range(3)),
        )
    return katman.resize((genislik, yukseklik), Image.BILINEAR)


def yuvarlak_maske(genislik, yukseklik, yaricap):
    maske = Image.new("L", (genislik, yukseklik), 0)
    ImageDraw.Draw(maske).rounded_rectangle(
        (0, 0, genislik - 1, yukseklik - 1), radius=yaricap, fill=255
    )
    return maske


def ikon_uret():
    tuval = Image.new("RGBA", (BOY, BOY), (0, 0, 0, 0))

    # --- Zemin: hafif yuvarlatılmış kare, dikey degrade ---------------------
    yaricap = round(BOY * 0.22)
    zemin = dikey_gecis(GECE_UST, GECE_ALT, BOY, BOY).convert("RGBA")
    tuval.paste(zemin, (0, 0), yuvarlak_maske(BOY, BOY, yaricap))

    # --- İnce kehribar çerçeve ---------------------------------------------
    cizim = ImageDraw.Draw(tuval)
    kalinlik = round(BOY * 0.012)
    cizim.rounded_rectangle(
        (kalinlik // 2, kalinlik // 2, BOY - 1 - kalinlik // 2, BOY - 1 - kalinlik // 2),
        radius=yaricap,
        outline=(*KENAR, 110),
        width=kalinlik,
    )

    # --- "N" monogramı: iki dikey gövde + çapraz bant ----------------------
    sol = round(BOY * 0.26)
    govde = round(BOY * 0.105)
    sag = round(BOY * 0.74) - govde
    ust = round(BOY * 0.265)
    alt = round(BOY * 0.735)
    bant = round(BOY * 0.108)

    n_katmani = Image.new("RGBA", (BOY, BOY), (0, 0, 0, 0))
    n_cizim = ImageDraw.Draw(n_katmani)
    n_cizim.rectangle((sol, ust, sol + govde, alt), fill=(255, 255, 255, 255))
    n_cizim.rectangle((sag, ust, sag + govde, alt), fill=(255, 255, 255, 255))
    n_cizim.polygon(
        [
            (sol + govde, ust),
            (sol + govde + bant, ust),
            (sag + govde, alt),
            (sag, alt),
            (sag, alt - bant),
            (sol + govde, ust + bant),
        ],
        fill=(255, 255, 255, 255),
    )
    # Çapraz bandın üst ucu gövdeyle kaynaşsın: köşe yumuşatması.
    n_cizim.polygon(
        [
            (sol + govde, ust),
            (sol + govde + bant, ust),
            (sol + govde + bant, ust + bant),
        ],
        fill=(255, 255, 255, 255),
    )

    kehribar = dikey_gecis(KEHRIBAR_UST, KEHRIBAR_ALT, BOY, BOY).convert("RGBA")
    tuval.paste(kehribar, (0, 0), n_katmani.split()[3])

    return tuval


def main():
    VARLIK.mkdir(exist_ok=True)
    buyuk = ikon_uret()

    png = VARLIK / "nexus_256.png"
    buyuk.resize((256, 256), Image.LANCZOS).save(png, "PNG")

    boyutlar = [16, 24, 32, 48, 64, 128, 256]
    kareler = [buyuk.resize((b, b), Image.LANCZOS) for b in boyutlar]
    ico = VARLIK / "nexus.ico"
    kareler[-1].save(ico, format="ICO", sizes=[(b, b) for b in boyutlar])

    print(f"yazildi: {ico} ({ico.stat().st_size} bayt)")
    print(f"yazildi: {png} ({png.stat().st_size} bayt)")


if __name__ == "__main__":
    main()
