"""Sektör başına örnek fotoğraf indirir (Wikimedia Commons, CC/PD lisanslı).

Çıktı: varliklar/foto/<aile>/1.webp ... ve varliklar/foto/kaynak.json
Manifest her kare için atıf satırını tutar; demo altbilgisinde gösterilir.

Kullanım: python araclar/stok_foto_indir.py
"""

import json
import pathlib
import subprocess
import tempfile
import urllib.parse
import urllib.request

KOK = pathlib.Path(__file__).resolve().parent.parent
HEDEF = KOK / "varliklar" / "foto"

UA = {"User-Agent": "demo-site-photo-fetcher/1.0 (yerel masaustu uygulamasi)"}

# Demo ailesi -> Commons arama terimi. İfadeler bilinçli olarak genel: amaç
# işletmenin kendi fotoğrafı değil, o sektörün profesyonel görünen kareleri.
AILELER = {
    "menu": "restaurant plate food served",
    "kafe": "coffee shop interior cafe",
    "unlu": "bakery bread counter",
    "urun": "boutique shop interior shelves",
    "galeri": "hair salon interior",
    "klinik": "dental clinic treatment room",
    "otel": "hotel room interior bed",
    "egitim": "classroom students desks",
    "ofis": "modern office interior workspace",
    "servis": "auto repair garage car",
    "eglence": "live music stage concert lights",
}

IZINLI = ("cc0", "cc by", "cc-by", "public domain", "pd-", "cc sa")
KARE_SAYISI = 3


def commons_ara(terim: str, adet: int) -> list[dict]:
    sorgu = {
        "action": "query",
        "format": "json",
        "generator": "search",
        "gsrsearch": f"filetype:bitmap {terim}",
        "gsrnamespace": "6",
        "gsrlimit": str(adet * 5),
        "prop": "imageinfo",
        "iiprop": "url|size|extmetadata|mime",
        "iiurlwidth": "1400",
    }
    adres = "https://commons.wikimedia.org/w/api.php?" + urllib.parse.urlencode(sorgu)
    with urllib.request.urlopen(urllib.request.Request(adres, headers=UA), timeout=40) as yanit:
        veri = json.load(yanit)
    return list(veri.get("query", {}).get("pages", {}).values())


def atif_metni(meta: dict) -> str | None:
    def alan(ad: str) -> str:
        return str(meta.get(ad, {}).get("value", "")).strip()

    lisans = alan("LicenseShortName")
    if not any(izin in lisans.lower() for izin in IZINLI):
        return None
    sanatci = alan("Artist")
    # HTML etiketlerini at ("<a href=...>Ad</a>")
    temiz, icerde = [], False
    for karakter in sanatci:
        if karakter == "<":
            icerde = True
        elif karakter == ">":
            icerde = False
        elif not icerde:
            temiz.append(karakter)
    sanatci = " ".join("".join(temiz).split())
    if not sanatci:
        return None
    return f"{sanatci} · {lisans} · Wikimedia Commons"


def webp_yaz(kaynak: bytes, hedef: pathlib.Path) -> bool:
    with tempfile.NamedTemporaryFile(suffix=".img", delete=False) as gecici:
        gecici.write(kaynak)
        yol = gecici.name
    try:
        subprocess.run(
            [
                "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
                "-i", yol,
                "-vf", "scale='min(1200,iw)':-2",
                "-quality", "72",
                str(hedef),
            ],
            check=True,
            timeout=120,
        )
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired, FileNotFoundError):
        pathlib.Path(yol).unlink(missing_ok=True)
        return False
    pathlib.Path(yol).unlink(missing_ok=True)
    return hedef.exists() and hedef.stat().st_size > 8000


def main() -> None:
    manifest: dict[str, list[dict]] = {}
    for aile, terim in AILELER.items():
        klasor = HEDEF / aile
        klasor.mkdir(parents=True, exist_ok=True)
        kayitlar: list[dict] = []
        try:
            sayfalar = commons_ara(terim, KARE_SAYISI)
        except Exception as hata:  # ağ hatası: aileyi atla, diğerleri devam etsin
            print(f"[!] {aile}: arama başarısız ({hata})")
            continue
        for sayfa in sayfalar:
            if len(kayitlar) >= KARE_SAYISI:
                break
            bilgi = (sayfa.get("imageinfo") or [{}])[0]
            if bilgi.get("mime") not in ("image/jpeg", "image/png"):
                continue
            if int(bilgi.get("width", 0)) < 1000:
                continue
            atif = atif_metni(bilgi.get("extmetadata", {}))
            if not atif:
                continue
            url = bilgi.get("thumburl") or bilgi.get("url")
            if not url:
                continue
            try:
                with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=60) as y:
                    icerik = y.read()
            except Exception:
                continue
            dosya_adi = f"{len(kayitlar) + 1}.webp"
            if not webp_yaz(icerik, klasor / dosya_adi):
                continue
            kayitlar.append({"dosya": dosya_adi, "atif": atif, "kaynak": bilgi.get("descriptionurl", "")})
        manifest[aile] = kayitlar
        print(f"{aile}: {len(kayitlar)} foto")

    HEDEF.mkdir(parents=True, exist_ok=True)
    (HEDEF / "kaynak.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
