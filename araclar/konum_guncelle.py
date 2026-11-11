#!/usr/bin/env python3
"""İşletme konumunu tek noktadan günceller.

Kayıtlı pin (pinler.json), ana sayfa yapısal verisi ve 31 bölge sayfası
aynı koordinata çekilir. Varsayılan davranış denemedir; dosyalara yalnızca
`--yaz` verildiğinde dokunulur.

Kullanım:
    python araclar/konum_guncelle.py 38.7344 27.5681
    python araclar/konum_guncelle.py 38.7344 27.5681 --yaz
"""

from __future__ import annotations

import argparse
import glob
import json
import math
import re
import sys

PIN_DOSYASI = "pinler.json"
SAYFALAR = ["site/index.html", "site/sahneler.html"]
BOLGE_DOSYALARI = "bolgesel/saruhanliotokurtarma/*.html"
ISLETME = "Saruhanlı Oto Kurtarma"

# Türkiye kabası: yanlışlıkla ABD/Asya koordinatı girilmesin.
TR = (35.5, 42.5, 25.5, 45.0)


def mesafe_km(a: tuple[float, float], b: tuple[float, float]) -> float:
    """İki nokta arası büyük daire mesafesi (km)."""
    r = 6371.0
    en1, bo1 = map(math.radians, a)
    en2, bo2 = map(math.radians, b)
    d_en = en2 - en1
    d_bo = bo2 - bo1
    h = math.sin(d_en / 2) ** 2 + math.cos(en1) * math.cos(en2) * math.sin(d_bo / 2) ** 2
    return 2 * r * math.asin(math.sqrt(h))


def harita_linki(enlem: float, boylam: float) -> str:
    return f"https://www.google.com/maps/search/?api=1&query={enlem:.6f},{boylam:.6f}"


def geo(enlem: float, boylam: float) -> dict:
    return {"@type": "GeoCoordinates", "latitude": enlem, "longitude": boylam}


def sayfa_guncelle(yol: str, enlem: float, boylam: float, yaz: bool) -> bool:
    """Sayfadaki AutoRepair bloğuna geo + hasMap yazar."""
    s = open(yol, encoding="utf-8").read()
    for blok in re.finditer(r'<script type="application/ld\+json">\s*(\{.*?\})\s*</script>', s, re.S):
        try:
            veri = json.loads(blok.group(1))
        except json.JSONDecodeError:
            continue
        if veri.get("@type") != "AutoRepair":
            continue
        onceki = (veri.get("geo") or {}).get("latitude")
        veri["geo"] = geo(enlem, boylam)
        veri["hasMap"] = harita_linki(enlem, boylam)
        yeni = json.dumps(veri, ensure_ascii=False, separators=(",", ":"))
        s = s[: blok.start(1)] + yeni + s[blok.end(1):]
        if yaz:
            open(yol, "w", encoding="utf-8").write(s)
        print(f"  {yol:<44} geo: {onceki or 'yok'} -> {enlem:.6f}")
        return True
    print(f"  {yol:<44} AutoRepair bloğu yok, atlandı")
    return False


def rust_sabiti_guncelle(enlem: float, boylam: float, yaz: bool) -> bool:
    """Üreticideki ISLETME_KONUM sabitini aynı noktaya çeker."""
    yol = "src/bolgesel.rs"
    s = open(yol, encoding="utf-8").read()
    desen = r"pub const ISLETME_KONUM: \(f64, f64\) = \([^)]*\);"
    yeni, sayi = re.subn(
        desen, f"pub const ISLETME_KONUM: (f64, f64) = ({enlem:.6f}, {boylam:.6f});", s
    )
    if not sayi:
        print(f"  {yol:<44} ISLETME_KONUM bulunamadı, atlandı")
        return False
    if yaz:
        open(yol, "w", encoding="utf-8").write(yeni)
    print(f"  {yol:<44} ISLETME_KONUM -> {enlem:.6f}, {boylam:.6f}")
    return True


def pin_guncelle(enlem: float, boylam: float, yaz: bool) -> tuple[tuple[float, float] | None, str | None]:
    with open(PIN_DOSYASI, encoding="utf-8") as f:
        defter = json.load(f)
    kayitlar = defter.setdefault("kayitlar", {})
    eski: tuple[float, float] | None = None
    eski_anahtar: str | None = None
    for anahtar, kayit in kayitlar.items():
        if anahtar.startswith("google:") and ISLETME.split()[0] in kayit.get("ad", ""):
            eski_anahtar = anahtar
            if kayit.get("enlem") and kayit.get("boylam"):
                eski = (float(kayit["enlem"]), float(kayit["boylam"]))
            kayit["enlem"] = round(enlem, 6)
            kayit["boylam"] = round(boylam, 6)
            kayit["tarih"] = __import__("datetime").date.today().isoformat()
            break
    if eski_anahtar is None:
        print("  pinler.json: işletme kaydı bulunamadı")
        return eski, None

    yeni_anahtar = f"google:{enlem:.6f},{boylam:.6f}"
    if yeni_anahtar != eski_anahtar:
        yeniden = {}
        for anahtar, kayit in kayitlar.items():
            yeniden[yeni_anahtar if anahtar == eski_anahtar else anahtar] = kayit
        defter["kayitlar"] = yeniden
    if yaz:
        with open(PIN_DOSYASI, "w", encoding="utf-8") as f:
            json.dump(defter, f, ensure_ascii=False, indent=2)
            f.write("\n")
    print(f"  {PIN_DOSYASI:<44} {eski_anahtar} -> {yeni_anahtar}")
    return eski, yeni_anahtar


def main() -> int:
    ap = argparse.ArgumentParser(description="İşletme konumunu güncelle")
    ap.add_argument("enlem", type=float)
    ap.add_argument("boylam", type=float)
    ap.add_argument("--yaz", action="store_true", help="dosyalara yaz (yoksa yalnızca gösterir)")
    secim = ap.parse_args()

    enlem, boylam = secim.enlem, secim.boylam
    if not (-90 <= enlem <= 90 and -180 <= boylam <= 180):
        print("HATA: koordinat aralık dışında", file=sys.stderr)
        return 2
    if not (TR[0] <= enlem <= TR[1] and TR[2] <= boylam <= TR[3]):
        print("UYARI: bu nokta Türkiye sınırları dışında görünüyor; yine de devam ediliyor.")

    print(f"{'YAZILIYOR' if secim.yaz else 'DENEME (dosyalara dokunulmadı)'} · {enlem:.6f}, {boylam:.6f}")
    eski, _ = pin_guncelle(enlem, boylam, secim.yaz)

    if eski:
        fark = mesafe_km(eski, (enlem, boylam))
        print(f"  eski noktadan uzaklık: {fark:.1f} km")
        if fark > 5:
            print("  UYARI: 5 km'den büyük değişim — pin gerçekten taşınıyor mu, kontrol et.")

    rust_sabiti_guncelle(enlem, boylam, secim.yaz)

    sayi = 0
    for yol in SAYFALAR + sorted(glob.glob(BOLGE_DOSYALARI)):
        if sayfa_guncelle(yol, enlem, boylam, secim.yaz):
            sayi += 1
    print(f"  güncellenen sayfa: {sayi}")
    if not secim.yaz:
        print("Uygulamak için sonuna --yaz ekle.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
