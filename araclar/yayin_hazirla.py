"""Yayın klasörünü hazırlar: bolgesel sayfaları + premium ana sayfa + font/poster.

Kullanım: python araclar/yayin_hazirla.py <yayin-adresi>
Çıktı: yayin/public (Sites'a yüklenecek statik dosyalar) + yayin/index.js + yayin/deploy.json
"""
import os
import re
import shutil
import sys

KOK = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
KAYNAK = os.path.join(KOK, "bolgesel", "saruhanliotokurtarma")
HEDEF = os.path.join(KOK, "yayin")
PUBLIC = os.path.join(HEDEF, "public")

INDEXNOW_ANAHTAR = "9f2c7a41d8b64e0fa53c1d7e6b902c48"

# Yayındaki mevcut adres: alan adı bağlanınca tüm canonical/og:url/sitemap
# kayıtları verilen yeni adrese çevrilir.
ESKI_ADRES = "https://site-74ad7168ac5940b0a4e353051388ae97.freebuff.page"

SAYFALAR = [
    "index.html",
    "izmir-kemalpasa.html",
    "manisa-turgutlu.html",
    "manisa-saruhanli.html",
    "manisa-akhisar.html",
    "manisa-kirkagac.html",
    "manisa-soma.html",
    "balikesir-savastepe.html",
]


def yaz(kaynak: str, hedef: str, url: str) -> None:
    """Sayfayı kopyalar, içindeki adresleri verilen yayın adresine çevirir."""
    with open(kaynak, encoding="utf-8") as f:
        icerik = f.read()
    icerik = icerik.replace(ESKI_ADRES, url).replace("https://ornek-alan-adi.com", url)
    with open(hedef, "w", encoding="utf-8") as f:
        f.write(icerik)


def main() -> int:
    if len(sys.argv) < 2:
        print("kullanım: python araclar/yayin_hazirla.py <https://alan-adresi>")
        return 2
    url = sys.argv[1].rstrip("/")

    shutil.rmtree(HEDEF, ignore_errors=True)
    os.makedirs(PUBLIC, exist_ok=True)

    for ad in SAYFALAR:
        yaz(os.path.join(KAYNAK, ad), os.path.join(PUBLIC, ad), url)
    yaz(os.path.join(KAYNAK, "sitemap.xml"), os.path.join(PUBLIC, "sitemap.xml"), url)
    yaz(os.path.join(KAYNAK, "robots.txt"), os.path.join(PUBLIC, "robots.txt"), url)
    shutil.copytree(os.path.join(KOK, "site", "fonts"), os.path.join(PUBLIC, "fonts"))
    # Tanıtım sayfaları (canvas yol + telefon-önce /cekici)
    for ad in ("tanitim", "cekici"):
        yaz(os.path.join(KOK, "site", f"{ad}.html"), os.path.join(PUBLIC, f"{ad}.html"), url)

    # Koridor haritası görseli
    if os.path.isdir(os.path.join(KOK, "site", "img")):
        shutil.copytree(os.path.join(KOK, "site", "img"), os.path.join(PUBLIC, "img"))


    # Ana sayfa: her açılışta farklı 3B video ile açılan yeni tasarım.
    yaz(os.path.join(KOK, "site", "anasayfa.html"), os.path.join(PUBLIC, "index.html"), url)

    # Premium ana sayfa ayrı adreste yayınlanır.
    with open(os.path.join(KOK, "site", "index.html"), encoding="utf-8") as f:
        ana = f.read()
    ana = ana.replace(ESKI_ADRES, url).replace("https://ornek-alan-adi.com", url)
    ana = re.sub(
        r'<link rel="canonical" href="[^"]*">',
        f'<link rel="canonical" href="{url}/anasayfa">',
        ana,
    )
    if 'rel="canonical"' not in ana:
        ana = ana.replace(
            "</head>", f'<link rel="canonical" href="{url}/anasayfa">\n</head>', 1
        )
    with open(os.path.join(PUBLIC, "anasayfa.html"), "w", encoding="utf-8") as f:
        f.write(ana)

    # Sitemap: uzantısız adresler + ana sayfa kaydı.
    yol = os.path.join(PUBLIC, "sitemap.xml")
    with open(yol, encoding="utf-8") as f:
        sm = f.read().replace(".html</loc>", "</loc>")
    ek = []
    for ad, oncelik in (("anasayfa", "0.6"), ("tanitim", "0.7"), ("cekici", "0.9")):
        if ad not in sm:
            ek.append(
                f"  <url><loc>{url}/{ad}</loc><changefreq>monthly</changefreq>"
                f"<priority>{oncelik}</priority></url>"
            )
    if ek:
        sm = sm.replace("</urlset>", "\n".join(ek) + "\n</urlset>")
    with open(yol, "w", encoding="utf-8") as f:
        f.write(sm)

    with open(os.path.join(HEDEF, "index.js"), "w", encoding="utf-8") as f:
        f.write(
            "export default {\n"
            "  async fetch(request, env) {\n"
            "    return env.ASSETS.fetch(request);\n"
            "  }\n"
            "};\n"
        )
    with open(os.path.join(HEDEF, "deploy.json"), "w", encoding="utf-8") as f:
        f.write(
            '{"entrypoint":"index.js","compatibilityDate":"2026-11-11",'
            '"modules":[{"name":"index.js","type":"esm","path":"./index.js"}],'
            '"assetsDirectory":"./public"}'
        )

    # IndexNow anahtarı: Bing/Yandex bu dosyayı görüp URL bildirimlerini kabul eder.
    with open(os.path.join(PUBLIC, f"{INDEXNOW_ANAHTAR}.txt"), "w", encoding="utf-8") as f:
        f.write(INDEXNOW_ANAHTAR)

    adet = toplam = 0
    for kok, _, dosyalar in os.walk(PUBLIC):
        for d in dosyalar:
            adet += 1
            toplam += os.path.getsize(os.path.join(kok, d))
    print(f"yayin hazir: {adet} dosya, {toplam / 1024:.0f} KB, {url}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
