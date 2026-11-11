"""Canlı adreste kliplerin gerçekten oynadığını doğrular.

Her klip için bakılanlar:
  - ana sayfada `klipler` listesinde geçiyor mu,
  - `video/hero/<ad>.mp4` → 200 + `video/mp4` (+ boyut yerelle aynı mı),
  - `Range` isteği 206 dönüyor mu (tarayıcı atlama/oynatma için),
  - poster `video/hero/<ad>.jpg` → 200 + `image/jpeg`,
  - **dosya indirilip ffprobe ile çözülüyor mu**: h264, 960×540, ~6 sn, ses yok.

Kullanım:
    python araclar/canli_dogrula.py                 # iki adres
    python araclar/canli_dogrula.py --surge         # yalnız kısa adres
    python araclar/canli_dogrula.py --kisa          # indirmeden hızlı kontrol
    python araclar/canli_dogrula.py --adres https://... --klip kaza

Sorun varsa çıkış kodu 1, uyarılar (ör. Range desteği yok) kodu etkilemez.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from surge_hazirla import BASE, KANONIK  # noqa: E402  (yol düzeltmesinden sonra)

KLASOR = os.path.join("site", "video", "hero")
SURE = 6.0
SURE_TOLERANS = 0.6
UA = {"User-Agent": "canli-dogrula/1"}
AG_HATALARI = (urllib.error.URLError, TimeoutError, OSError)


def klip_adlari():
    """Ana sayfadaki `klipler` dizisinden adlar; yoksa klasörden."""
    sayfa = os.path.join("site", "anasayfa.html")
    adlar = []
    if os.path.exists(sayfa):
        with open(sayfa, encoding="utf-8") as f:
            adlar = re.findall(r"video/hero/([A-Za-z0-9_-]+)\.mp4", f.read())
    if not adlar and os.path.isdir(KLASOR):
        adlar = [f[:-4] for f in sorted(os.listdir(KLASOR)) if f.endswith(".mp4")]
    return list(dict.fromkeys(adlar))


def istek(url, yontem="GET", ek=None, zaman=30):
    basliklar = dict(UA, **(ek or {}))
    return urllib.request.urlopen(
        urllib.request.Request(url, method=yontem, headers=basliklar), timeout=zaman
    )


def kod(e):
    return f"HTTP {e.code}" if isinstance(e, urllib.error.HTTPError) else "bağlantı hatası"


def video_coz(url, ad="klip"):
    """Dosyayı indirip ffprobe ile oynatılabilir olduğunu doğrular: (ok, mesaj, KB)."""
    gecici = tempfile.mkdtemp(prefix="canli_")
    try:
        yol = os.path.join(gecici, f"{ad}.mp4")
        try:
            with istek(url) as y, open(yol, "wb") as f:
                shutil.copyfileobj(y, f)
        except AG_HATALARI as e:
            return False, f"indirilemedi ({kod(e)})", 0.0
        boyut = os.path.getsize(yol)
        if boyut < 20_000:
            return False, f"indirilen dosya çok küçük ({boyut} B)", 0.0
        try:
            v = subprocess.run(
                ["ffprobe", "-v", "error", "-print_format", "json",
                 "-show_format", "-show_streams", yol],
                capture_output=True, text=True,
            )
        except OSError:
            return False, "indirilen dosya çözülemiyor (ffprobe yok)", 0.0
        try:
            d = json.loads(v.stdout) if v.returncode == 0 else {}
        except ValueError:
            d = {}
        akis = next((s for s in d.get("streams", []) if s.get("codec_type") == "video"), None)
        if akis is None:
            return False, "indirilen dosya video olarak çözülemiyor", 0.0
        sure = float(d.get("format", {}).get("duration") or akis.get("duration") or 0)
        kusur = []
        if akis.get("codec_name") != "h264":
            kusur.append(f"codec {akis.get('codec_name')}")
        if (akis.get("width"), akis.get("height")) != (960, 540):
            kusur.append(f"{akis.get('width')}x{akis.get('height')}")
        if abs(sure - SURE) > SURE_TOLERANS:
            kusur.append(f"süre {sure:.2f} sn")
        if any(s.get("codec_type") == "audio" for s in d.get("streams", [])):
            kusur.append("ses var")
        if kusur:
            return False, "oynatılamaz: " + ", ".join(kusur), 0.0
        return True, f"oynatılabilir {sure:.2f} sn · 960x540 · {boyut / 1024:.0f} KB", boyut / 1024
    finally:
        shutil.rmtree(gecici, ignore_errors=True)


def adres_denetle(adres, adlar, indir=True):
    """Adresin denetim özetini döndürür; satır satır ve özet satırıyla yazar."""
    adres = adres.rstrip("/")
    print(f"\n=== {adres} ===")
    sorun = uyari = iyi = 0
    kb = 0.0

    try:
        with istek(adres + "/") as y:
            durum = y.status
            govde = y.read(600_000).decode("utf-8", "ignore")
        if durum != 200:
            print(f"✗ ana sayfa: HTTP {durum}")
            sorun += 1
        else:
            eksik = [a for a in adlar if f"video/hero/{a}.mp4" not in govde]
            if eksik:
                print(f"✗ ana sayfa: listede olmayan klip — {', '.join(eksik)}")
                sorun += 1
            else:
                print(f"✓ ana sayfa: 200 · {len(adlar)} klip listeleniyor")
    except AG_HATALARI as e:
        print(f"✗ ana sayfa: {kod(e)}")
        sorun += 1

    for ad in adlar:
        mp4 = f"{adres}/video/hero/{ad}.mp4"
        yerel = os.path.join(KLASOR, f"{ad}.mp4")
        notlar, kotu, uy = [], 0, 0

        try:
            with istek(mp4, "HEAD") as y:
                durum = y.status
                tur = (y.headers.get("content-type") or "").lower()
                boy = y.headers.get("content-length")
            notlar.append(str(durum))
            if durum != 200 or not tur.startswith("video/mp4"):
                kotu += 1
                notlar.append(f"content-type {tur or 'yok'}")
            if boy and os.path.exists(yerel) and int(boy) != os.path.getsize(yerel):
                kotu += 1
                notlar.append(f"boyut {boy} ≠ yerel {os.path.getsize(yerel)}")
        except AG_HATALARI as e:
            kotu += 1
            notlar.append(kod(e))

        if not kotu:
            try:
                with istek(mp4, "GET", {"Range": "bytes=0-1023"}) as y:
                    if y.status == 206:
                        notlar.append("Range 206")
                    else:
                        uy += 1
                        notlar.append("Range yok")
            except AG_HATALARI:
                uy += 1
                notlar.append("Range denenemedi")

        try:
            with istek(f"{adres}/video/hero/{ad}.jpg", "HEAD") as y:
                durum = y.status
                tur = (y.headers.get("content-type") or "").lower()
            if durum != 200 or not tur.startswith("image/jpeg"):
                kotu += 1
                notlar.append(f"poster {durum} {tur or 'yok'}")
        except AG_HATALARI as e:
            kotu += 1
            notlar.append(f"poster {kod(e)}")

        boyut_kb = 0.0
        if indir and not kotu:
            ok, mesaj, boyut_kb = video_coz(mp4, ad)
            notlar.append(mesaj)
            if not ok:
                kotu += 1
        elif os.path.exists(yerel):
            boyut_kb = os.path.getsize(yerel) / 1024

        sorun += kotu
        uyari += uy
        if not kotu:
            iyi += 1
            kb += boyut_kb
        isaret = "✗" if kotu else ("!" if uy else "✓")
        print(f"{isaret} {ad:16} " + " · ".join(notlar))

    print(f"özet: {iyi}/{len(adlar)} klip oynatılabilir · {kb / 1024:.2f} MB"
          + (f" · {uyari} uyarı" if uyari else ""))
    return {"adres": adres, "klip": len(adlar), "iyi": iyi, "kb": kb,
            "sorun": sorun, "uyari": uyari}


def main():
    p = argparse.ArgumentParser(description="Canlı adreste kliplerin oynadığını doğrular.")
    p.add_argument("--adres", action="append",
                   help="denetlenecek adres (birden çok verilebilir)")
    p.add_argument("--surge", action="store_true", help=f"yalnız kısa adres ({BASE})")
    p.add_argument("--kisa", action="store_true", help="dosyaları indirip çözmeden hızlı kontrol")
    p.add_argument("--klip", action="append", help="yalnız belirli klip (birden çok verilebilir)")
    args = p.parse_args()

    adresler = args.adres or ([BASE] if args.surge else [BASE, KANONIK])
    adlar = args.klip or klip_adlari()
    if not adlar:
        print("denetlenecek klip yok (site/video/hero boş ve ana sayfada liste yok)")
        return 1

    print(f"canlı kontrol: {len(adlar)} klip · {len(adresler)} adres"
          + (" · indirme yok" if args.kisa else " · dosyalar indirilip çözülüyor"))
    kayitlar = [adres_denetle(a, adlar, indir=not args.kisa) for a in adresler]
    toplam = sum(r["sorun"] for r in kayitlar)

    print("\n=== toplam ===")
    genislik = max(len(r["adres"]) for r in kayitlar) + 2
    for r in kayitlar:
        print(f"{r['adres']:<{genislik}}{r['iyi']:>2}/{r['klip']:<2} klip · "
              f"{r['kb'] / 1024:6.2f} MB · {r['sorun']} sorun"
              + (f" · {r['uyari']} uyarı" if r["uyari"] else ""))
    print(f"{len(kayitlar)} adres · {sum(r['iyi'] for r in kayitlar)} klip doğrulandı · "
          f"{sum(r['kb'] for r in kayitlar) / 1024:.2f} MB · {toplam} sorun")

    if toplam:
        print(f"\n{toplam} sorun var — adreste eksik/bozuk dosya ya da oynatılamayan klip.")
        return 1
    print("\ntamam: klipler canlıda oynatılabilir durumda.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print("\nkesildi")
        raise SystemExit(130)
