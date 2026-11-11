"""Yayın adresi değiştiğinde tüm canonical/sitemap kayıtlarını günceller.

Panelde kısa bir alt alan adı alındıktan (veya alan adı bağlandıktan) sonra:

    python araclar/adres_yenile.py                     # adayları yokla
    python araclar/adres_yenile.py https://yeni-adres  # doğrula + paketle

Yoklama modunda her adresin gerçekten bu siteyi sunup sunmadığına bakar
(başlık + telefon işareti). Paketleme modunda adres doğrulanır, sonra
canonical, og:url, JSON-LD, sitemap ve robots yeni adrese çevrilir.
"""
import subprocess
import sys
import urllib.error
import urllib.request

KOK_YOL = "araclar/yayin_hazirla.py"

MEVCUT = "https://site-74ad7168ac5940b0a4e353051388ae97.freebuff.page"

# Panelde denemeye değer kısa adlar.
ADAYLAR = [
    MEVCUT,
    "https://saruhanlio.freebuff.page",
    "https://saruhanliotokurtarma.freebuff.page",
    "https://saruhanli-otokurtarma.freebuff.page",
]

# Sitenin kimlik işaretleri (ana sayfa HTML'inde birlikte bulunurlar).
ISARETLER = ("Saruhanlı Oto Kurtarma", "905071299439")


def bizim_site(adres: str, zaman: int = 15) -> bool:
    """Adres 200 dönüyor ve sayfa bu işletmenin sayfası mı?"""
    kok = adres.rstrip("/") + "/"
    istek = urllib.request.Request(kok, headers={"User-Agent": "adres-yenile/1"})
    try:
        with urllib.request.urlopen(istek, timeout=zaman) as yanit:
            if yanit.status != 200:
                return False
            govde = yanit.read(400_000).decode("utf-8", "ignore")
    except (urllib.error.URLError, TimeoutError, OSError):
        return False
    return all(isaret in govde for isaret in ISARETLER)


def yokla() -> int:
    print("Aday adresler (yalnızca bu siteyi sunanlar 'BİZİM SİTE' olur):\n")
    bulunan = []
    for adres in ADAYLAR:
        bizim = bizim_site(adres)
        if bizim:
            bulunan.append(adres)
        print(f"  {adres:62} {'BİZİM SİTE' if bizim else '-'}")
    print()
    yeni = [a for a in bulunan if a.rstrip("/") != MEVCUT]
    if yeni:
        print("Yeni adres yayında:", yeni[0])
        print(f"  python araclar/adres_yenile.py {yeni[0]}")
    else:
        print("Yeni adres bulunamadı. Panelde kısa adı kaydettikten sonra bu")
        print("komutu tekrar çalıştır ya da adresi doğrudan ver:")
        print("  python araclar/adres_yenile.py https://yeni-adres")
    return 0


def paketle(adres: str) -> int:
    adres = adres.rstrip("/")
    print(f"Doğrulanıyor: {adres} ...", flush=True)
    if not bizim_site(adres):
        print("HATA: bu adres siteyi sunmuyor (404, farklı site veya erişilemiyor).", flush=True)
        print("Önce panelde adresi bağla/yayınla, sonra tekrar dene.")
        return 1
    print("Adres doğru. Yayın klasörü yeniden üretiliyor ...", flush=True)
    sonuc = subprocess.run(
        [sys.executable, KOK_YOL, adres], check=False, text=True
    )
    if sonuc.returncode != 0:
        print("HATA: yayin_hazirla.py başarısız oldu.")
        return sonuc.returncode
    print()
    print("Canonical, og:url, JSON-LD, sitemap ve robots yeni adrese çevrildi.")
    print("Sonraki adım: yayin/deploy.json ile yayına al.")
    return 0


def main() -> int:
    if len(sys.argv) > 1 and sys.argv[1].startswith("http"):
        return paketle(sys.argv[1])
    return yokla()


if __name__ == "__main__":
    raise SystemExit(main())
