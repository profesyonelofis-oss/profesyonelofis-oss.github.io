"""Nexus Enterprise: OSM/Overpass katmanini main.rs'ten soker.

Silinenler: Overpass istek zinciri, OSM sorgu kurucu, OSM yanit isleyici,
Nominatim konum arama, ilce/sonuc onbellegi, hibrit takviye yardimcilari,
Kaynak secimi ve Sektor.osm eslemesi.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

YOL = Path("src/main.rs")


def oku() -> list[str]:
    with YOL.open("r", encoding="utf-8", newline="") as dosya:
        return dosya.readlines()


def yaz(satirlar: list[str]) -> None:
    with YOL.open("w", encoding="utf-8", newline="") as dosya:
        dosya.writelines(satirlar)


def kapanis_bul(satirlar: list[str], bas: int, acik: str, kapali: str) -> int:
    """bas satirindan baslayarak acik/kapali parantez dengesine gore kapanis."""
    derinlik = 0
    gorundu = False
    for i in range(bas, len(satirlar)):
        for karakter in satirlar[i]:
            if karakter == acik:
                derinlik += 1
                gorundu = True
            elif karakter == kapali:
                derinlik -= 1
                if gorundu and derinlik == 0:
                    return i
    raise SystemExit(f"kapanis bulunamadi: satir {bas + 1}")


def yorum_basi(satirlar: list[str], bas: int, girinti: str) -> int:
    """Doc comment / attribute satirlarini da silmek icin geriye yuru."""
    while bas > 0:
        onceki = satirlar[bas - 1].strip()
        if onceki.startswith("///") or onceki.startswith("#["):
            bas -= 1
            continue
        if onceki.startswith("//") and not onceki.startswith("///"):
            bas -= 1
            continue
        break
    return bas


def fn_sil(satirlar: list[str], ad: str) -> bool:
    desen = re.compile(rf"^(\s*)fn {re.escape(ad)}\b")
    for i, satir in enumerate(satirlar):
        eslesme = desen.match(satir)
        if not eslesme:
            continue
        bas = yorum_basi(satirlar, i, eslesme.group(1))
        son = kapanis_bul(satirlar, i, "{", "}")
        del satirlar[bas : son + 1]
        return True
    return False


def oge_sil(satirlar: list[str], onek: str) -> bool:
    """struct/enum/impl/type/const gibi ogeleri adiyla siler."""
    desen = re.compile(rf"^\s*(pub(\(crate\))? )?{onek}\b")
    for i, satir in enumerate(satirlar):
        if not desen.match(satir):
            continue
        bas = yorum_basi(satirlar, i, "")
        if "{" in satir[satir.find(onek) :]:
            son = kapanis_bul(satirlar, i, "{", "}")
        else:
            son = i
            while son < len(satirlar) and not satirlar[son].rstrip().endswith(";"):
                son += 1
        del satirlar[bas : son + 1]
        return True
    return False


def satir_sil(satirlar: list[str], desen: str) -> int:
    kalip = re.compile(desen)
    once = len(satirlar)
    satirlar[:] = [satir for satir in satirlar if not kalip.search(satir)]
    return once - len(satirlar)


def main() -> int:
    satirlar = oku()
    silinen_fn = []
    for ad in [
        "overpass_isle",
        "overpass_sorgusu",
        "overpass_istek",
        "overpass_istek_zincir",
        "sitesiz_isletmeler",
        "yedek_sunucu_bildir",
        "ilce_taramasi",
        "etkin_kaynak",
        "osm_kurallari",
        "etiket",
        "adres_metni",
        "kayit_dolulugu",
        "foto_etiketi",
        "ad_benzerligi",
        "ad_normal",
        "ayni_isletme",
        "telefonlari_tamamla",
        "mesafe_metre",
        "takviye_gerekli",
        "google_takviye_ekle",
        "tarama_calistir",
        "tarama_il_geneli",
        "tarama_tek_ilce",
        "konum_bul",
        "kapsama_yaricapi",
        "kota_pili",
    ]:
        if fn_sil(satirlar, ad):
            silinen_fn.append(ad)
        else:
            print(f"UYARI: fn bulunamadi: {ad}", file=sys.stderr)

    for onek in [
        "enum Kaynak",
        "impl Kaynak",
        "enum IlceTaramasi",
        "impl IlceTaramasi",
        "struct SonucKaydi",
        "struct SonucOnbellekKaydi",
        "struct SonucOnbellek",
        "impl SonucOnbellek",
        "type TaramaParcasi",
        "const NOMINATIM_URL",
        "const OVERPASS_SUNUCULARI",
        "const OVERPASS_BUTCE",
        "const OVERPASS_DENEMESI",
        "const WEB_SITESI_ETIKETLERI",
        "const SONUC_ONBELLEK_DOSYASI",
        "const SONUC_TTL_GUN",
        "const GOOGLE_TARAMA_SINIRI",
        "const GOOGLE_TAKVIYE_ESIGI",
        "const ESLESME_MESAFESI_M",
        "const ESLESME_BENZERLIGI",
        "const ESLESME_EN_KISA_AD",
        "const NIYETLI_KOTA",
    ]:
        if not oge_sil(satirlar, onek):
            print(f"UYARI: oge bulunamadi: {onek}", file=sys.stderr)

    # Derleme zamani assert'i takviye esigine bagliydi.
    satir_sil(satirlar, r"const _: \(\) = assert!\(GOOGLE_TAKVIYE_ESIGI")
    # Sektor katalogundaki OSM etiket eslemesi.
    n = satir_sil(satirlar, r"^        osm: &\[")
    print(f"Sektor osm satiri silindi: {n}")
    # Sektor struct'indaki alan ve doc satiri.
    satir_sil(satirlar, r"^\s*/// Overpass sorgusunda kullanilan")
    satir_sil(satirlar, r"^\s*osm: &'static \[\(&'static str, &'static str\)\],")
    # Konum kayitlarindaki Overpass alan kimligi.
    n = satir_sil(satirlar, r"\balan_id\b")
    print(f"alan_id satiri silindi: {n}")
    # Kaynak secimi LeadApp alanlari.
    satir_sil(satirlar, r"^\s*kaynak: Kaynak,$")
    satir_sil(satirlar, r"^\s*kaynak: Kaynak::Hibrit,$")

    yaz(satirlar)
    print("Silinen fonksiyonlar:", ", ".join(silinen_fn))
    print("Kalan satir:", len(satirlar))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
