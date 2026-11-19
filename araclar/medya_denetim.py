#!/usr/bin/env python3
"""Medya arşivi denetimi (fotoğraf + video).

Üç katman:
1. Manifest (`KAYNAK.json`) doğrulaması: kayıt/dosya eşleşmesi, kaynak ve lisans.
2. Alakalılık: başlık, sektörün grup İngilizce anahtar kelimeleriyle eşleşiyor mu;
   `baslik_uygun()` yasak kelimelerinden sızan kayıt var mı.
3. Teknik: rastgele seçilen sektörlerde ffprobe ile bozukluk, çözünürlük,
   süre ve ses durumu.

Salt okunur; hiçbir dosyayı silmez. Sonuçlar `medya_denetim_raporu.md` ve
`medya_denetim.json` dosyalarına yazılır.
"""

from __future__ import annotations

import hashlib
import json
import random
import re
import subprocess
import sys
import unicodedata
from pathlib import Path

KOK = Path(__file__).resolve().parent.parent
ARSIV = KOK / "media" / "stock"
RAPOR_MD = KOK / "medya_denetim_raporu.md"
RAPOR_JSON = KOK / "medya_denetim.json"
TOHUM = 20261105
VIDEO_MAKS_BAYT = 7 * 1024 * 1024

CEVIRI = str.maketrans("ıİşŞğĞüÜöÖçÇâÂ", "iIsSgGuUoOcCaA")


def sade(metin: str) -> str:
    """Türkçe ve aksanlı harfleri ASCII'ye indirir, küçültür, sıkıştırır."""
    metin = metin.translate(CEVIRI)
    metin = unicodedata.normalize("NFKD", metin)
    metin = "".join(k for k in metin if not unicodedata.combining(k))
    return re.sub(r"[^a-z0-9]+", " ", metin.lower()).strip()


def slug(metin: str) -> str:
    return re.sub(r"_+", "_", re.sub(r"[^a-z0-9]+", "_", sade(metin))).strip("_")


def sektorler() -> list[tuple[str, str]]:
    """main.rs içindeki SEKTORLER listesini (ad, grup) olarak okur."""
    metin = (KOK / "src" / "main.rs").read_text(encoding="utf-8", errors="replace")
    bas = metin.find("pub(crate) const SEKTORLER: &[Sektor] = &[")
    if bas < 0:
        raise SystemExit("SEKTORLER listesi bulunamadı")
    parca = metin[bas : metin.find("\n];", bas)]
    return [
        (ad, grup)
        for ad, grup in re.findall(r'ad:\s*"([^"]+)",\s*grup:\s*"([^"]+)"', parca)
    ]


def grup_ingilizce() -> dict[str, str]:
    """sektor_medya::ingilizce eşlemesini Rust kaynağından çıkarır."""
    metin = (KOK / "src" / "sektor_medya.rs").read_text(encoding="utf-8", errors="replace")
    bas = metin.find("pub fn ingilizce(grup: &str) -> &'static str {")
    if bas < 0:
        return {}
    parca = metin[bas : metin.find("\n}", bas)]
    return dict(re.findall(r'"([^"]+)"\s*=>\s*"([^"]+)"', parca))


def yasak_kelimeler() -> tuple[set[str], list[str]]:
    """medya_tedarik::baslik_uygun içindeki YASAK listelerini okur."""
    metin = (KOK / "src" / "medya_tedarik.rs").read_text(encoding="utf-8", errors="replace")
    kelimeler: set[str] = set()
    ifadeler: list[str] = []
    for ad, hedef in (("const YASAK: &[&str] = &[", kelimeler), ("const YASAK_IFADE: &[&str] = &[", ifadeler)):
        bas = metin.find(ad)
        if bas < 0:
            continue
        parca = metin[bas : metin.find("];", bas)]
        bulunan = re.findall(r'"([^"]+)"', parca)
        if isinstance(hedef, set):
            hedef.update(bulunan)
        else:
            hedef.extend(bulunan)
    return kelimeler, ifadeler


def genel_kelimeler() -> set[str]:
    """yedek_kelimeler() içindeki sektörden bağımsız yedek aramalar."""
    metin = (KOK / "src" / "medya_tedarik.rs").read_text(encoding="utf-8", errors="replace")
    bas = metin.find("fn yedek_kelimeler(grup: &str) -> Vec<String> {")
    if bas < 0:
        return set()
    parca = metin[bas : metin.find("\n}", bas)]
    return {
        kelime
        for ifade in re.findall(r'"([^"]+)"', parca)
        for kelime in sade(ifade).split()
        if len(kelime) >= 3
    }


def probe(yol: Path) -> tuple[dict | None, str]:
    try:
        sonuc = subprocess.run(
            [
                "ffprobe",
                "-v",
                "error",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
                str(yol),
            ],
            capture_output=True,
            text=True,
            timeout=40,
        )
    except (OSError, subprocess.TimeoutExpired) as hata:
        return None, str(hata)
    if sonuc.returncode != 0:
        return None, sonuc.stderr.strip()[:200]
    try:
        return json.loads(sonuc.stdout), ""
    except json.JSONDecodeError as hata:
        return None, f"ffprobe çıktısı okunamadı: {hata}"


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    tanimlar = sektorler()
    ingilizce = grup_ingilizce()
    yasak, yasak_ifade = yasak_kelimeler()
    genel = genel_kelimeler()
    if not ARSIV.is_dir():
        raise SystemExit(f"arşiv yok: {ARSIV}")

    klasorler = {p.name: p for p in ARSIV.iterdir() if p.is_dir()}
    dosyadan_sektor = {}
    for ad, klasor in klasorler.items():
        manifest = klasor / "KAYNAK.json"
        if not manifest.is_file():
            continue
        try:
            veri = json.loads(manifest.read_text(encoding="utf-8", errors="replace"))
        except json.JSONDecodeError:
            continue
        if isinstance(veri.get("sektor"), str):
            dosyadan_sektor[veri["sektor"]] = ad

    kayit_sayisi = 0
    video_sayisi = 0
    foto_sayisi = 0
    eksik_dosya: list[dict] = []
    yetim_dosya: list[str] = []
    kucuk_dosya: list[str] = []
    kucuk_yol: list[Path] = []
    kaynaksiz: list[dict] = []
    basliksiz: list[dict] = []
    yasak_ihlal: list[dict] = []
    zayif: list[dict] = []
    genel_yedek: list[dict] = []
    supheli: list[dict] = []
    kopyalar: dict[str, list[str]] = {}
    sektor_ist: list[dict] = []
    manifest_eksik_sektor: list[str] = []

    for ad, grup in tanimlar:
        klasor_adi = dosyadan_sektor.get(ad) or slug(ad)
        klasor = klasorler.get(klasor_adi)
        if klasor is None or not (klasor / "KAYNAK.json").is_file():
            manifest_eksik_sektor.append(ad)
            continue
        try:
            veri = json.loads((klasor / "KAYNAK.json").read_text(encoding="utf-8", errors="replace"))
        except json.JSONDecodeError:
            manifest_eksik_sektor.append(ad)
            continue

        grup_kelime = {k for k in sade(ingilizce.get(grup, "business")).split() if len(k) >= 3}
        sektor_kelime = {k for k in sade(ad).split() if len(k) >= 3}
        anahtar = grup_kelime | sektor_kelime
        kayitli: set[str] = set()
        toplam = 0
        eslesen = 0

        for anahtar_ad in ("videolar", "fotolar"):
            for kayit in veri.get(anahtar_ad) or []:
                toplam += 1
                kayit_sayisi += 1
                dosya = str(kayit.get("dosya") or "")
                baslik = str(kayit.get("baslik") or "")
                kayitli.add(dosya)
                if not baslik or not kayit.get("kaynak") or not kayit.get("lisans"):
                    kaynaksiz.append({"sektor": ad, "dosya": dosya, "baslik": baslik})
                if dosya and not (klasor / dosya).is_file():
                    eksik_dosya.append({"sektor": ad, "dosya": dosya})
                if not baslik:
                    # Eski kayıtlar (baslik alanı eklenmeden önce) doğrulanamaz.
                    basliksiz.append({"sektor": ad, "dosya": dosya})
                    continue
                sade_baslik = sade(baslik)
                kelimeler = set(sade_baslik.split())
                eslesti = any(
                    (k in sade_baslik)
                    if len(k) >= 4
                    else (k in kelimeler or f"{k}s" in kelimeler)
                    for k in anahtar
                )
                if eslesti:
                    eslesen += 1
                else:
                    kayit_ozet = {
                        "sektor": ad,
                        "grup": grup,
                        "baslik": baslik,
                        "dosya": dosya,
                    }
                    zayif.append(kayit_ozet)
                    genel_mi = any(
                        (k in sade_baslik)
                        if len(k) >= 4
                        else (k in kelimeler or f"{k}s" in kelimeler)
                        for k in genel
                    )
                    (genel_yedek if genel_mi else supheli).append(kayit_ozet)
                if kelimeler & yasak or any(i in sade_baslik for i in yasak_ifade):
                    yasak_ihlal.append({"sektor": ad, "baslik": baslik, "dosya": dosya})

        for alt, onek, uzanti in (("video", "hero_", ".mp4"), ("photos", "img_", ".webp")):
            for dosya in sorted((klasor / alt).glob(f"{onek}*{uzanti}")):
                if f"{alt}/{dosya.name}" not in kayitli:
                    yetim_dosya.append(f"{klasor.name}/{alt}/{dosya.name}")
                boyut = dosya.stat().st_size
                ozet = hashlib.sha256()
                try:
                    with dosya.open("rb") as akis:
                        for parca in iter(lambda: akis.read(1 << 20), b""):
                            ozet.update(parca)
                    parmak_izi = ozet.hexdigest()
                    kopyalar.setdefault(parmak_izi, []).append(
                        f"{klasor.name}/{alt}/{dosya.name}"
                    )
                except OSError:
                    pass
                if (uzanti == ".mp4" and boyut < 50_000) or (
                    uzanti == ".webp" and boyut < 5_000
                ):
                    kucuk_dosya.append(f"{klasor.name}/{alt}/{dosya.name} ({boyut} bayt)")
                    kucuk_yol.append(dosya)
                if uzanti == ".mp4":
                    video_sayisi += 1
                else:
                    foto_sayisi += 1

        if toplam:
            sektor_ist.append(
                {
                    "sektor": ad,
                    "grup": grup,
                    "kayit": toplam,
                    "eslesen": eslesen,
                    "oran": round(100.0 * eslesen / toplam, 1),
                }
            )

    sektor_ist.sort(key=lambda s: s["oran"])

    # --- Teknik örneklem -------------------------------------------------
    rastgele = random.Random(TOHUM)
    ornek_aday = [s for s in sektor_ist if s["kayit"]]
    ornek_sektor = rastgele.sample(ornek_aday, min(40, len(ornek_aday)))
    teknik: list[dict] = []
    for sektor in ornek_sektor:
        klasor = ARSIV / (dosyadan_sektor.get(sektor["sektor"]) or slug(sektor["sektor"]))
        for alt, onek, uzanti, tur, adet in (
            ("video", "hero_", ".mp4", "video", 3),
            ("photos", "img_", ".webp", "foto", 3),
        ):
            dosyalar = sorted((klasor / alt).glob(f"{onek}*{uzanti}"))
            for dosya in rastgele.sample(dosyalar, min(adet, len(dosyalar))):
                bilgi, hata = probe(dosya)
                satir = {
                    "sektor": sektor["sektor"],
                    "dosya": f"{alt}/{dosya.name}",
                    "tur": tur,
                    "bayt": dosya.stat().st_size,
                    "sorun": [],
                }
                if bilgi is None:
                    satir["sorun"].append(f"bozuk/okunamadı: {hata}")
                    teknik.append(satir)
                    continue
                akislar = bilgi.get("streams") or []
                gorsel = next((a for a in akislar if a.get("codec_type") == "video"), None)
                ses = [a for a in akislar if a.get("codec_type") == "audio"]
                sure = float((bilgi.get("format") or {}).get("duration") or 0)
                if gorsel is None:
                    satir["sorun"].append("görüntü akışı yok")
                else:
                    satir["genislik"] = gorsel.get("width")
                    satir["yukseklik"] = gorsel.get("height")
                    satir["codec"] = gorsel.get("codec_name")
                    if tur == "video":
                        if (gorsel.get("width"), gorsel.get("height")) != (1280, 720):
                            satir["sorun"].append(
                                f"çözünürlük {gorsel.get('width')}x{gorsel.get('height')} ≠ 1280x720"
                            )
                        if gorsel.get("codec_name") != "h264":
                            satir["sorun"].append(f"codec {gorsel.get('codec_name')} ≠ h264")
                        if sure > 12.2 or sure <= 0:
                            satir["sorun"].append(f"süre {sure:.1f} sn (hedef ≤12)")
                        if ses:
                            satir["sorun"].append("ses akışı var (sessiz olmalı)")
                        if dosya.stat().st_size > VIDEO_MAKS_BAYT:
                            satir["sorun"].append("7 MB sınırı aşıldı")
                    else:
                        if gorsel.get("codec_name") != "webp":
                            satir["sorun"].append(f"codec {gorsel.get('codec_name')} ≠ webp")
                        if (gorsel.get("width") or 0) < 1200:
                            satir["sorun"].append("genişlik < 1200 px")
                satir["sure"] = round(sure, 2)
                teknik.append(satir)

    kucuk_detay = []
    for yol in kucuk_yol:
        bilgi, hata = probe(yol)
        kucuk_detay.append(
            {
                "dosya": str(yol.relative_to(ARSIV)).replace("\\", "/"),
                "bayt": yol.stat().st_size,
                "ffprobe": "ok" if bilgi else (hata or "okunamadı"),
            }
        )

    tekrar_gruplari = [yollar for yollar in kopyalar.values() if len(yollar) > 1]
    tekrar_gruplari.sort(key=len, reverse=True)
    tekrar_fazla = sum(len(g) - 1 for g in tekrar_gruplari)

    teknik_sorunlu = [t for t in teknik if t["sorun"]]
    dogrulanan = kayit_sayisi - len(basliksiz)
    zayif_oran = round(100.0 * (dogrulanan - len(zayif)) / dogrulanan, 1) if dogrulanan else 0.0

    rapor = {
        "tarih": None,
        "sektor_tanim": len(tanimlar),
        "sektor_klasor": len(klasorler),
        "manifestli_sektor": len(tanimlar) - len(manifest_eksik_sektor),
        "manifest_eksik_sektor": manifest_eksik_sektor,
        "kayit": kayit_sayisi,
        "video": video_sayisi,
        "foto": foto_sayisi,
        "eksik_dosya": len(eksik_dosya),
        "yetim_dosya": len(yetim_dosya),
        "kaynaksiz": len(kaynaksiz),
        "basliksiz": len(basliksiz),
        "dogrulanan": dogrulanan,
        "kucuk_dosya": len(kucuk_dosya),
        "yasak_ihlal": len(yasak_ihlal),
        "zayif_eslesme": len(zayif),
        "genel_yedek": len(genel_yedek),
        "supheli": len(supheli),
        "alakalilik_orani": zayif_oran,
        "teknik_ornek": len(teknik),
        "teknik_sorunlu": len(teknik_sorunlu),
        "tekrar_grup": len(tekrar_gruplari),
        "tekrar_fazla_dosya": tekrar_fazla,
        "tekrar_ornek": tekrar_gruplari[:15],
        "en_zayif_sektorler": sektor_ist[:15],
        "zayif_ornek": zayif[:25],
        "yasak_ornek": yasak_ihlal[:25],
        "eksik_ornek": eksik_dosya[:25],
        "kucuk_ornek": kucuk_dosya[:25],
        "kucuk_detay": kucuk_detay,
        "supheli_ornek": supheli[:40],
        "genel_yedek_ornek": genel_yedek[:15],
        "yetim_ornek": yetim_dosya[:40],
        "basliksiz_ornek": basliksiz[:25],
        "teknik": teknik,
    }
    RAPOR_JSON.write_text(json.dumps(rapor, ensure_ascii=False, indent=2), encoding="utf-8")

    # --- Markdown ---------------------------------------------------------
    satirlar = [
        "# Medya Arşivi Denetim Raporu",
        "",
        f"- Sektör tanımı: **{len(tanimlar)}** · diskte klasör: **{len(klasorler)}** · "
        f"manifestli: **{rapor['manifestli_sektor']}**",
        f"- Kayıt: **{kayit_sayisi}** (video {video_sayisi} · fotoğraf {foto_sayisi})",
        f"- Başlığı olmayan (doğrulanamayan): **{len(basliksiz)}** · doğrulanan: **{dogrulanan}**",
        f"- Başlık-alakalılık: **%{zayif_oran}** en az bir anahtar kelime içeriyor "
        f"({len(zayif)} kayıt zayıf: {len(genel_yedek)} genel yedek görüntü, "
        f"{len(supheli)} şüpheli)",
        f"- Yasak filtresi ihlali: **{len(yasak_ihlal)}** · küçük/boş dosya: **{len(kucuk_dosya)}**",
        f"- Manifestte olup diskte olmayan: **{len(eksik_dosya)}** · "
        f"diskte olup manifestte olmayan: **{len(yetim_dosya)}** · kaynaksız/lisanssız: **{len(kaynaksiz)}**",
        f"- Teknik örneklem: **{len(teknik)}** dosya, sorunlu **{len(teknik_sorunlu)}**",
        f"- Aynı dosyanın birden fazla sektörde kopyası: **{len(tekrar_gruplari)}** grup · "
        f"{tekrar_fazla} fazla kopya",
        "",
        "## En zayıf eşleşen sektörler",
        "",
        "| Sektör | Grup | Kayıt | Eşleşen | Oran |",
        "| --- | --- | ---: | ---: | ---: |",
    ]
    for s in sektor_ist[:15]:
        satirlar.append(
            f"| {s['sektor']} | {s['grup']} | {s['kayit']} | {s['eslesen']} | %{s['oran']} |"
        )
    satirlar += ["", "## Yasak başlık ihlalleri", ""]
    satirlar += (
        [f"- {i['sektor']}: {i['baslik']}" for i in yasak_ihlal[:40]] or ["- yok"]
    )
    satirlar += ["", "## Tekrarlanan dosyalar (aynı klip birden çok sektörde)", ""]
    satirlar += (
        [
            "- " + " · ".join(g[:5]) + (f" (+{len(g) - 5})" if len(g) > 5 else "")
            for g in tekrar_gruplari[:15]
        ]
        or ["- yok"]
    )
    satirlar += ["", "## Şüpheli kayıtlar (sektör ve genel yedek kelimesi yok)", ""]
    satirlar += (
        [f"- {s['sektor']} ({s['grup']}): {s['baslik']}" for s in supheli[:40]]
        or ["- yok"]
    )
    satirlar += ["", "## Küçük/bozuk dosyalar", ""]
    satirlar += (
        [f"- {k['dosya']}: {k['bayt']} bayt · ffprobe {k['ffprobe']}" for k in kucuk_detay]
        or ["- yok"]
    )
    satirlar += ["", "## Teknik sorunlar (örneklem)", ""]
    satirlar += (
        [
            f"- {t['sektor']} {t['dosya']}: {'; '.join(t['sorun'])}"
            for t in teknik_sorunlu[:40]
        ]
        or ["- yok"]
    )
    satirlar += ["", "## Zayıf eşleşme örnekleri", ""]
    satirlar += (
        [f"- {z['sektor']} ({z['grup']}): {z['baslik']}" for z in zayif[:25]] or ["- yok"]
    )
    satirlar += ["", "## Başlığı olmayan kayıtlar (doğrulanamıyor)", ""]
    satirlar += (
        [f"- {b['sektor']}: {b['dosya']}" for b in basliksiz[:25]] or ["- yok"]
    )
    satirlar += ["", "## Manifest eksik sektörler", ""]
    satirlar += [f"- {a}" for a in manifest_eksik_sektor[:40]] or ["- yok"]
    satirlar += ["", "## Diskte fazla dosyalar", ""]
    satirlar += [f"- {a}" for a in yetim_dosya[:40]] or ["- yok"]
    RAPOR_MD.write_text("\n".join(satirlar) + "\n", encoding="utf-8")

    print(f"sektör tanımı        : {len(tanimlar)}")
    print(f"diskte klasör        : {len(klasorler)} · manifestli {rapor['manifestli_sektor']}")
    print(f"kayıt                : {kayit_sayisi} (video {video_sayisi} · foto {foto_sayisi})")
    print(f"başlıksız/doğrulanamayan: {len(basliksiz)} · doğrulanan {dogrulanan}")
    print(f"alakalılık           : %{zayif_oran} · zayıf {len(zayif)}")
    print(f"küçük/boş dosya      : {len(kucuk_dosya)}")
    print(f"genel yedek görüntü  : {len(genel_yedek)} · şüpheli {len(supheli)}")
    print(f"tekrar eden grup    : {len(tekrar_gruplari)} · fazla kopya {tekrar_fazla}")
    print(f"yasak ihlal          : {len(yasak_ihlal)}")
    print(f"eksik dosya          : {len(eksik_dosya)} · yetim {len(yetim_dosya)} · kaynaksız {len(kaynaksiz)}")
    print(f"teknik örnek         : {len(teknik)} · sorunlu {len(teknik_sorunlu)}")
    print(f"rapor                : {RAPOR_MD}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
