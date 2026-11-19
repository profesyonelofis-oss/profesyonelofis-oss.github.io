"""Tek komutla yayın: siteyi üret, GSC doğrulamasını koru, GitHub Pages'e gönder.

Kullanım:
    python araclar/yayinla.py                     # üret + GitHub Pages'e yayınla
    python araclar/yayinla.py --surge             # ayrıca surge aynasını da güncelle
    python araclar/yayinla.py --mesaj "Yeni fiyat tablosu"
    python araclar/yayinla.py --kuru              # sadece üret, push etme (deneme)

Ne yapar:
  1. `araclar/yayin_hazirla.py` ile siteyi GitHub Pages adresine göre yeniden üretir.
  2. `dogrulama/` içindeki GSC doğrulama dosyasını/meta etiketini pakete ekler
     (Search Console doğrulaması böylece her yayında korunur).
  3. Siteyi `profesyonelofis-oss/profesyonelofis-oss.github.io` deposuna gönderir.
  4. GitHub Pages derlemesinin bitmesini bekler ve canlı adresleri kontrol eder.
  5. `--surge` verilirse aynı içeriği `saruhanliotokurtarma.surge.sh` aynasına yükler.
"""
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

KOK = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

import gsc_dogrula as gsc  # noqa: E402  (aynı klasördeki yardımcı)

SURGE_ALAN = "saruhanliotokurtarma.surge.sh"
SURGE_KLASOR = KOK / "_gecici" / "surge-yayin"


def dogrulama_ozeti() -> str:
    dosyalar = sorted(gsc.DOGRULAMA.glob("google*.html"))
    meta = gsc.META_DOSYA.is_file() and gsc.META_DOSYA.read_text(encoding="utf-8").strip()
    if not dosyalar and not meta:
        return "yok (GSC henüz eklenmedi)"
    parcalar = [d.name for d in dosyalar]
    if meta:
        parcalar.append("meta etiketi")
    return ", ".join(parcalar)


def paket_ozeti() -> str:
    adet = toplam = 0
    for kok, _, dosyalar in os.walk(gsc.PUBLIC):
        for ad in dosyalar:
            adet += 1
            toplam += os.path.getsize(os.path.join(kok, ad))
    return f"{adet} dosya, {toplam / 1024:.0f} KB"


def gh_var() -> bool:
    return shutil.which("gh") is not None


def pages_bekle(sure_sn: int = 240) -> str:
    """GitHub Pages derlemesi bitene kadar bekler, son durumu döndürür."""
    if not gh_var():
        return "gh yok (durum sorgulanamadı)"
    depo = f"{gsc.KULLANICI}/{gsc.REPO}"
    bitis = time.time() + sure_sn
    durum = "bilinmiyor"
    while time.time() < bitis:
        sonuc = subprocess.run(
            ["gh", "api", f"repos/{depo}/pages/builds/latest", "--jq", ".status"],
            capture_output=True, text=True,
        )
        durum = (sonuc.stdout or "").strip() or "bilinmiyor"
        if durum == "built":
            break
        time.sleep(8)
    return durum


def canli_kontrol() -> bool:
    """Ana sayfa ve çekici sayfasının yayında olduğunu doğrular."""
    tamam = True
    for yol in ("/", "/cekici", "/robots.txt", "/sitemap.xml"):
        kod, _ = gsc.getir(f"{gsc.ADRES}{yol}")
        tamam &= kod == 200
        print(f"   {yol:<14} {kod}")
    return tamam


def surge_yayinla() -> bool:
    """Aynı içeriği surge aynasına yükler (jeton _gecici/surge_token.txt)."""
    jeton = KOK / "_gecici" / "surge_token.txt"
    if not jeton.is_file():
        print("   surge jetonu yok: _gecici/surge_token.txt")
        return False
    shutil.rmtree(SURGE_KLASOR, ignore_errors=True)
    shutil.copytree(gsc.PUBLIC, SURGE_KLASOR)
    for ad in ("CNAME", ".git"):
        yol = SURGE_KLASOR / ad
        if yol.is_dir():
            shutil.rmtree(yol, ignore_errors=True)
        elif yol.exists():
            yol.unlink()
    env = dict(os.environ)
    env["SURGE_LOGIN"] = "kuzeykaraoglu2017@gmail.com"
    env["SURGE_TOKEN"] = jeton.read_text(encoding="utf-8").strip()
    npx = shutil.which("npx.cmd") or shutil.which("npx") or "npx"
    komut = f'"{npx}" --yes surge ./_gecici/surge-yayin {SURGE_ALAN}'
    sonuc = subprocess.run(komut, shell=True, env=env, cwd=str(KOK))
    return sonuc.returncode == 0


def main() -> int:
    ap = argparse.ArgumentParser(description="Siteyi üret ve GitHub Pages'e yayınla")
    ap.add_argument("--surge", action="store_true", help="surge aynasını da güncelle")
    ap.add_argument("--mesaj", default="", help="commit mesajı")
    ap.add_argument("--kuru", action="store_true", help="push etme, sadece üret")
    secim = ap.parse_args()

    print("1/4 · site üretiliyor…")
    gsc.siteyi_uret()
    eklenen = gsc.dogrulama_uygula()
    print(f"   paket: {paket_ozeti()} · canonical: {gsc.ADRES}")
    print(f"   GSC doğrulaması: {dogrulama_ozeti()}")
    if eklenen:
        print(f"   pakete eklendi: {', '.join(eklenen)}")

    if secim.kuru:
        print("kuru çalışma: push yapılmadı.")
        return 0

    print("2/4 · GitHub Pages'e gönderiliyor…")
    ozet = gsc.yayinla(secim.mesaj or "Site güncellendi")
    print(f"   commit: {ozet}")

    print("3/4 · Pages derlemesi bekleniyor…")
    print(f"   durum: {pages_bekle()}")

    print("4/4 · canlı kontrol")
    tamam = canli_kontrol()
    if dogrulama_ozeti().startswith("yok") is False:
        gsc.kontrol(bekle=60)

    if secim.surge:
        print("ek · surge aynası güncelleniyor…")
        print("   " + ("tamam" if surge_yayinla() else "atlandı/hata"))

    print("\nsonuç:", "yayında" if tamam else "canlı kontrolde eksik var — yukarıdaki kodlara bakın")
    return 0 if tamam else 1


if __name__ == "__main__":
    raise SystemExit(main())
