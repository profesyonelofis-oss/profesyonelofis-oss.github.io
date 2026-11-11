"""Surge.sh aynası: siteyi kısa adreste yayınlar.

Not: surge'un ücretsiz alt alan adlarında `/robots.txt` her zaman
`Disallow: /` döndürüyor; bu yüzden arama motoru yetkisi (canonical, og:url,
sitemap) **freebuff.page** adresinde kalır. Surge adresi müşteriye
verilecek kısa adres olarak çalışır.

Kullanım:
    python araclar/surge_hazirla.py            # yalnız hazırla
    python araclar/surge_hazirla.py --yayinla  # hazırla + surge'a yükle
"""
import os
import re
import shutil
import subprocess
import sys

# Arama motoruna yetkili adres (robots.txt taramaya açık olan host).
KANONIK = "https://site-74ad7168ac5940b0a4e353051388ae97.freebuff.page"
# Kısa, paylaşılabilir adres.
BASE = "https://saruhanliotokurtarma.surge.sh"
HEDEF = "surge_site"
PUBLIC = os.path.join("yayin", "public")

SAYFALAR = [
    "anasayfa",
    "tanitim",
    "cekici",
    "izmir-kemalpasa",
    "manisa-turgutlu",
    "manisa-saruhanli",
    "manisa-akhisar",
    "manisa-kirkagac",
    "manisa-soma",
    "balikesir-savastepe",
]


def baglantilari_duzelt() -> None:
    """Surge klasör yapısı sonda eğik çizgi ister; iç bağlantıları ona uydur."""
    degisen = 0
    for kok, _, dosyalar in os.walk(HEDEF):
        for ad in dosyalar:
            if not ad.endswith(".html"):
                continue
            yol = os.path.join(kok, ad)
            with open(yol, encoding="utf-8") as f:
                metin = f.read()
            yeni = metin
            for s in SAYFALAR:
                yeni = re.sub(rf'href="/{s}(?=[\"#?])', f'href="/{s}/"', yeni)
            if yeni != metin:
                with open(yol, "w", encoding="utf-8") as f:
                    f.write(yeni)
                degisen += 1
    print(f"iç bağlantı düzeltmesi: {degisen} dosya")


def surge_klasoru() -> None:
    shutil.rmtree(HEDEF, ignore_errors=True)
    shutil.copytree(PUBLIC, HEDEF)
    for ad in sorted(os.listdir(HEDEF)):
        if not ad.endswith(".html") or ad == "index.html":
            continue
        taban = ad[:-5]
        hedef = os.path.join(HEDEF, taban)
        os.makedirs(hedef, exist_ok=True)
        shutil.move(os.path.join(HEDEF, ad), os.path.join(hedef, "index.html"))
    adet = sum(1 for k, _, d in os.walk(HEDEF) for x in d if x == "index.html")
    print(f"surge klasörü hazır: {adet} sayfa")


def main() -> int:
    # Yetkili adres freebuff.page: canonical/sitemap oraya yazılır.
    subprocess.run([sys.executable, "araclar/yayin_hazirla.py", KANONIK], check=True)
    surge_klasoru()
    baglantilari_duzelt()
    if "--yayinla" not in sys.argv:
        print("hazır. yüklemek için: python araclar/surge_hazirla.py --yayinla")
        return 0

    env = dict(os.environ)
    env["SURGE_LOGIN"] = "kuzeykaraoglu2017@gmail.com"
    token_yolu = os.path.join("_gecici", "surge_token.txt")
    if os.path.exists(token_yolu) and not env.get("SURGE_TOKEN"):
        with open(token_yolu, encoding="utf-8") as f:
            env["SURGE_TOKEN"] = f.read().strip()
    # Windows'ta npx bir .cmd betiğidir; doğrudan çalıştırılamaz.
    npx = shutil.which("npx.cmd") or shutil.which("npx") or "npx"
    komut = f'"{npx}" --yes surge ./{HEDEF} saruhanliotokurtarma.surge.sh'
    sonuc = subprocess.run(komut, env=env, shell=True, check=False)
    return sonuc.returncode


if __name__ == "__main__":
    raise SystemExit(main())
