"""Google Search Console doğrulamasını siteye ekler ve GitHub Pages'e yayınlar.

Kullanım:
    # 1) GSC'nin indirdiği HTML dosyasıyla (en kolay yol)
    python araclar/gsc_dogrula.py --dosya "C:\\Users\\...\\google1a2b3c4d.html"

    # 2) HTML etiketi (meta) yöntemiyle
    python araclar/gsc_dogrula.py --meta "gsc-token-buraya"

    # Sadece hazırla, yayınlama (deneme)
    python araclar/gsc_dogrula.py --meta "deneme" --kuru

    # Yayındaki doğrulama dosyasını kontrol et
    python araclar/gsc_dogrula.py --kontrol

Yaptığı iş:
  * doğrulama dosyasını `dogrulama/` klasörüne kopyalar (sonraki yayınlarda kaybolmaz),
  * siteyi GitHub Pages adresine göre yeniden üretir,
  * dosyayı/meta etiketini yayın paketine ekler,
  * `profesyonelofis-oss/profesyonelofis-oss.github.io` deposuna push eder,
  * doğrulama dosyasının canlıda 200 döndüğünü kontrol eder.
"""
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

KOK = Path(__file__).resolve().parent.parent
KULLANICI = "profesyonelofis-oss"
REPO = f"{KULLANICI}.github.io"
ADRES = f"https://{REPO}"
DOGRULAMA = KOK / "dogrulama"
PUBLIC = KOK / "yayin" / "public"
GHP = KOK / "_gecici" / "gh-pages"
META_DOSYA = DOGRULAMA / "meta.txt"
# surge, gh-pages klasörüne CNAME bırakıyor; GitHub'a giderse özel alan adı sanılır.
YASAK = {"CNAME"}


def kos(komut: list[str], cwd: Path | None = None) -> None:
    sonuc = subprocess.run(komut, cwd=str(cwd or KOK))
    if sonuc.returncode != 0:
        raise SystemExit(f"komut başarısız: {' '.join(komut)}")


def siteyi_uret() -> None:
    """Canonical adres GitHub Pages olacak şekilde yayın paketini üretir."""
    kos([sys.executable, "araclar/yayin_hazirla.py", ADRES])
    if not (PUBLIC / "index.html").is_file():
        raise SystemExit("yayin/public/index.html üretilemedi")


def dogrulama_uygula() -> list[str]:
    """dogrulama/ içindeki dosya ve meta etiketini yayın paketine ekler."""
    DOGRULAMA.mkdir(exist_ok=True)
    eklenen: list[str] = []

    for dosya in sorted(DOGRULAMA.glob("google*.html")):
        shutil.copy2(dosya, PUBLIC / dosya.name)
        eklenen.append(dosya.name)

    if META_DOSYA.is_file():
        token = META_DOSYA.read_text(encoding="utf-8").strip()
        if token:
            etiket = f'<meta name="google-site-verification" content="{token}">'
            yol = PUBLIC / "index.html"
            html = yol.read_text(encoding="utf-8")
            if 'name="google-site-verification"' not in html:
                html = html.replace("</head>", etiket + "\n</head>", 1)
                yol.write_text(html, encoding="utf-8")
            eklenen.append("meta etiketi (ana sayfa)")

    return eklenen


def yayinla(mesaj: str = "GitHub Pages yayını") -> str:
    """yayin/public içeriğini GitHub Pages deposuna gönderir."""
    # Windows'ta .git kilitli kalabildiği için klasörü silmek yerine içini boşaltıyoruz.
    GHP.mkdir(parents=True, exist_ok=True)
    for giris in list(GHP.iterdir()):
        if giris.name == ".git":
            continue
        try:
            shutil.rmtree(giris) if giris.is_dir() else giris.unlink()
        except OSError:
            pass
    shutil.copytree(PUBLIC, GHP, dirs_exist_ok=True)
    for ad in YASAK:
        (GHP / ad).unlink(missing_ok=True)
    (GHP / ".nojekyll").touch()

    if not (GHP / ".git").is_dir():
        kos(["git", "init", "-q", "-b", "main"], cwd=GHP)
    kos(["git", "config", "user.name", "Profesyonel Ofis"], cwd=GHP)
    kos(["git", "config", "user.email", "profesyonelofis@gmail.com"], cwd=GHP)
    kos(["git", "add", "-A"], cwd=GHP)
    # değişiklik yoksa commit hata verir; --allow-empty ile geçiyoruz
    kos(["git", "commit", "-q", "--allow-empty", "-m", mesaj or "GitHub Pages yayını"], cwd=GHP)

    uzak = f"https://github.com/{KULLANICI}/{REPO}.git"
    uzaklar = subprocess.run(
        ["git", "remote"], cwd=str(GHP), capture_output=True, text=True
    ).stdout.split()
    if "origin" in uzaklar:
        kos(["git", "remote", "set-url", "origin", uzak], cwd=GHP)
    else:
        kos(["git", "remote", "add", "origin", uzak], cwd=GHP)
    kos(["git", "push", "-q", "-f", "origin", "main"], cwd=GHP)

    return subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"], cwd=str(GHP),
        capture_output=True, text=True,
    ).stdout.strip()


def getir(url: str) -> tuple[int, str]:
    istek = urllib.request.Request(url, headers={"User-Agent": "gsc-dogrula/1.0"})
    try:
        with urllib.request.urlopen(istek, timeout=20) as yanit:
            return yanit.status, yanit.read(4000).decode("utf-8", "replace")
    except urllib.error.HTTPError as hata:
        return hata.code, ""
    except urllib.error.URLError:
        return 0, ""


def kontrol(bekle: int = 90) -> int:
    """Doğrulama dosyası ve meta etiketinin canlıda göründüğünü kontrol eder."""
    dosyalar = sorted(DOGRULAMA.glob("google*.html"))
    token = META_DOSYA.read_text(encoding="utf-8").strip() if META_DOSYA.is_file() else ""
    if not dosyalar and not token:
        print("doğrulama dosyası/meta yok. önce --dosya veya --meta ile ekleyin.")
        return 2

    son = (0, "")
    for deneme in range(1, 13):
        basarili = True
        for dosya in dosyalar:
            kod, _ = getir(f"{ADRES}/{dosya.name}")
            print(f"  {deneme:2}. deneme · /{dosya.name} → {kod}")
            basarili &= kod == 200
        if token:
            kod, html = getir(f"{ADRES}/")
            var = token in html
            print(f"  {deneme:2}. deneme · meta etiketi → {kod} · {'var' if var else 'yok'}")
            basarili &= kod == 200 and var
        if basarili:
            print("doğrulama canlıda hazır.")
            return 0
        son = (deneme, "")
        time.sleep(max(5, bekle // 12))

    print("doğrulama henüz canlıda görünmüyor; GitHub Pages derlemesi sürebilir.")
    return 1


def main() -> int:
    ap = argparse.ArgumentParser(description="Google Search Console doğrulaması + GitHub Pages yayını")
    ap.add_argument("--dosya", help="GSC'den indirilen google....html dosyasının yolu")
    ap.add_argument("--meta", help="GSC'nin verdiği HTML etiketi içerik değeri (content=...)")
    ap.add_argument("--kuru", action="store_true", help="yayınlama, sadece paketi hazırla")
    ap.add_argument("--kontrol", action="store_true", help="yayındaki doğrulamayı kontrol et")
    secim = ap.parse_args()

    if secim.kontrol:
        return kontrol()

    if not secim.dosya and not secim.meta:
        ap.print_help()
        return 2

    DOGRULAMA.mkdir(exist_ok=True)

    if secim.dosya:
        kaynak = Path(secim.dosya).expanduser()
        if not kaynak.is_file():
            print(f"dosya bulunamadı: {kaynak}")
            return 2
        if not kaynak.name.startswith("google") or kaynak.suffix != ".html":
            print("beklenen dosya adı google....html — GSC'nin verdiği dosyayı seçin.")
            return 2
        shutil.copy2(kaynak, DOGRULAMA / kaynak.name)
        print(f"doğrulama dosyası alındı: {kaynak.name}")

    if secim.meta:
        META_DOSYA.write_text(secim.meta.strip(), encoding="utf-8")
        print("meta token kaydedildi: dogrulama/meta.txt")

    siteyi_uret()
    eklenen = dogrulama_uygula()
    print("yayın paketine eklendi:", ", ".join(eklenen) or "(değişiklik yok)")

    if secim.kuru:
        print("kuru çalışma: push yapılmadı.")
        return 0

    ozet = yayinla()
    print(f"GitHub Pages'e gönderildi: {ozet}")
    return kontrol()


if __name__ == "__main__":
    raise SystemExit(main())
