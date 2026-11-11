"""Kaldırılan özelliklere bağlı testleri temizler (dize sabitlerini atlar)."""

import re
from pathlib import Path

YOL = Path("src/main.rs")
DIZE = re.compile(r'"(?:[^"\\]|\\.)*"')
KARAKTER = re.compile(r"'(?:[^'\\]|\\.)'")
YORUM = re.compile(r"//.*$")
TEST = re.compile(r"^    fn (\w+)")


def temizle(satir: str) -> str:
    satir = DIZE.sub('""', satir)
    satir = KARAKTER.sub("''", satir)
    return YORUM.sub("", satir)


def kapanis(satirlar: list[str], bas: int) -> int:
    derinlik = 0
    basladi = False
    for i in range(bas, len(satirlar)):
        temiz = temizle(satirlar[i])
        derinlik += temiz.count("{") - temiz.count("}")
        if "{" in temiz:
            basladi = True
        if basladi and derinlik == 0:
            return i
    raise SystemExit(f"kapanis yok: satir {bas + 1}")


def main() -> None:
    satirlar = YOL.read_text(encoding="utf-8").splitlines(keepends=True)
    mod = next(i for i, s in enumerate(satirlar) if s.startswith("mod testler"))
    yasak = (
        "overpass_",
        "OSM",
        "Kaynak::",
        "etkin_kaynak",
        "ESLESME_",
        "GOOGLE_TAKVIYE_ESIGI",
        "NIYETLI_KOTA",
        "ad_benzerligi",
        "takviye_gerekli",
        "google_takviye_ekle",
        "telefonlari_tamamla",
        "ayni_isletme",
        "ilce_taramasi",
        "sitesiz_isletmeler",
        "SonucKaydi",
        "KONUM_DOSYASI",
        "overpass_isle",
        "foto_etiketi",
        "osm_kurallari",
        "overpass-api.de",
        "kayit_dolulugu(&",
        "Overpass",
    )
    silinen = []
    i = mod
    while i < len(satirlar):
        eslesme = TEST.match(satirlar[i])
        if not eslesme:
            i += 1
            continue
        bas = i
        while bas > mod and (
            satirlar[bas - 1].strip().startswith("#[")
            or satirlar[bas - 1].strip().startswith("///")
        ):
            bas -= 1
        son = kapanis(satirlar, i)
        govde = "".join(satirlar[i : son + 1])
        if any(anahtar in govde for anahtar in yasak):
            silinen.append(eslesme.group(1))
            del satirlar[bas : son + 1]
            i = bas
            continue
        i = son + 1

    YOL.write_text("".join(satirlar), encoding="utf-8", newline="")
    print("silinen test:", len(silinen))
    for ad in silinen:
        print("  -", ad)


main()
