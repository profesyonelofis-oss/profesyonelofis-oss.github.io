#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Sektör içi tekrar temizliği.

    python medya_temizle.py              # yalnız rapor (kuru çalışma)
    python medya_temizle.py --uygula     # fazlalıkları media/tekrar'a taşır

Politika (sektör korumacı):
  * Her sektör kendi medyasını korur; sektörler arası kopyalar SİLİNMEZ,
    yalnız sayılır (bir sektörün tek fotoğrafı başka sektörle aynıysa o
    sektör boş kalmasın).
  * Yalnız aynı sektörün kendi içindeki tekrarlar tekilleştirilir:
      - birebir aynı dosyalar (SHA-256, aynı boyuttakiler arasında)
      - %92+ benzer fotoğraflar (8x8 ortalama hash, ≤5 bit fark)
  * Her grupta en büyük dosya korunur, fazlalıklar `media/tekrar/<sektör>/…`
    altına taşınır (silinmez, geri alınabilir).
  * Video afişleri (`video/*.webp`) aday sayılmaz: video kartları afişsiz kalmasın.
"""

from __future__ import annotations

import argparse
import hashlib
import shutil
import sys
from collections import Counter, defaultdict
from pathlib import Path

VIDEO_UZANTI = {".mp4", ".webm", ".mov", ".mkv", ".m4v"}
FOTO_UZANTI = {".webp", ".jpg", ".jpeg", ".png", ".gif", ".avif"}
KOK = Path(__file__).resolve().parent
STOK = KOK / "media" / "stock"
TEKRAR = KOK / "media" / "tekrar"
BENZER_BIT = 5

try:  # Windows konsolu UTF-8 olmayabilir; Türkçe çıktı bozulmasın.
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass

try:
    from PIL import Image
except ImportError:  # görsel hash opsiyonel; yoksa yalnız birebir tekrar
    Image = None


def medyalar() -> list[tuple[Path, str, bool]]:
    """(yol, sektör, video_mu) — yalnız photos/ ve video/ altındaki medya."""
    liste: list[tuple[Path, str, bool]] = []
    if not STOK.is_dir():
        return liste
    for sektor in sorted(p for p in STOK.iterdir() if p.is_dir()):
        for alt in ("photos", "video"):
            klasor = sektor / alt
            if not klasor.is_dir():
                continue
            for dosya in sorted(klasor.iterdir()):
                if not dosya.is_file():
                    continue
                uzanti = dosya.suffix.lower()
                if uzanti in VIDEO_UZANTI:
                    liste.append((dosya, sektor.name, True))
                elif uzanti in FOTO_UZANTI:
                    # `video/` içindeki webp bir afiştir; aday değil.
                    if alt == "video":
                        continue
                    liste.append((dosya, sektor.name, False))
    return liste


def sha256(yol: Path) -> str:
    ozet = hashlib.sha256()
    with yol.open("rb") as dosya:
        for parca in iter(lambda: dosya.read(1024 * 1024), b""):
            ozet.update(parca)
    return ozet.hexdigest()


def gorsel_hash(yol: Path) -> int | None:
    """8x8 ortalama hash: 64 bit parmak izi."""
    if Image is None:
        return None
    try:
        with Image.open(yol) as gorsel:
            kucuk = gorsel.convert("L").resize((8, 8))
            pikseller = list(kucuk.tobytes())
    except Exception:
        return None
    if not pikseller:
        return None
    ortalama = sum(pikseller) / len(pikseller)
    deger = 0
    for sira, piksel in enumerate(pikseller):
        if piksel >= ortalama:
            deger |= 1 << sira
    return deger


def benzerlik(a: int, b: int) -> int:
    return 100 - bin(a ^ b).count("1") * 100 // 64


def main() -> int:
    ayar = argparse.ArgumentParser(description="Sektör içi tekrar temizliği")
    ayar.add_argument("--uygula", action="store_true", help="fazlalıkları media/tekrar'a taşı")
    ayar.add_argument("--sinir", type=int, default=BENZER_BIT, help="benzerlik için en fazla farklı bit")
    secim = ayar.parse_args()

    dosyalar = medyalar()
    if not dosyalar:
        print("[TEKİLLE] arşivde medya yok")
        return 0
    toplam_mb = sum(yol.stat().st_size for yol, _, _ in dosyalar) / (1024 * 1024)
    yontem = "SHA-256 + aHash" if Image is not None else "yalnız SHA-256 (Pillow yok)"
    print(
        f"[TEKİLLE] {len(dosyalar)} dosya · {toplam_mb:.0f} MB · yöntem {yontem} · "
        f"sektör içi · benzerlik <={secim.sinir}/64 bit"
    )

    # 1) Sektör içi birebir tekrar (aynı boyuttakiler hash'lenir).
    boyuta_gore: dict[tuple[str, int], list[tuple[Path, bool]]] = defaultdict(list)
    for yol, sektor, video in dosyalar:
        try:
            boyuta_gore[(sektor, yol.stat().st_size)].append((yol, video))
        except OSError:
            continue
    tekrarlar: list[tuple[Path, str, str, int]] = []  # (yol, sektör, neden, bayt)
    for (_sektor, boyut), grup in boyuta_gore.items():
        if len(grup) < 2:
            continue
        hash_gruplari: dict[str, list[tuple[Path, bool]]] = defaultdict(list)
        for yol, video in grup:
            hash_gruplari[sha256(yol)].append((yol, video))
        for ayni in hash_gruplari.values():
            if len(ayni) < 2:
                continue
            ayni.sort(key=lambda oge: (-oge[0].stat().st_size, str(oge[0])))
            tutulan = ayni[0][0]
            sektor = tutulan.relative_to(STOK).parts[0]
            for yol, _video in ayni[1:]:
                tekrarlar.append(
                    (yol, sektor, f"birebir aynı: {tutulan.relative_to(STOK)}", boyut)
                )
    birebir = len(tekrarlar)

    # 2) Sektör içi görsel benzerlik (büyükten küçüğe; en iyi kopya korunur).
    tekrar_yollari = {oge[0] for oge in tekrarlar}
    sektor_foto: dict[str, list[tuple[Path, int]]] = defaultdict(list)
    for yol, sektor, video in dosyalar:
        if video or yol in tekrar_yollari:
            continue
        try:
            sektor_foto[sektor].append((yol, yol.stat().st_size))
        except OSError:
            continue
    for sektor, foto in sektor_foto.items():
        foto.sort(key=lambda oge: (-oge[1], str(oge[0])))
        korunan: list[tuple[int, Path]] = []
        for yol, boyut in foto:
            deger = gorsel_hash(yol)
            if deger is None:
                continue
            for eski_deger, eski in korunan:
                if bin(deger ^ eski_deger).count("1") <= secim.sinir:
                    tekrarlar.append(
                        (yol, sektor, f"%{benzerlik(deger, eski_deger)} benzer: {eski.relative_to(STOK)}", boyut)
                    )
                    break
            else:
                korunan.append((deger, yol))

    # Bilgi: sektörler arası birebir kopyalar (taşınmaz, sektörler boşalmasın).
    sektor_hash: dict[int, set[str]] = defaultdict(set)
    for yol, sektor, _video in dosyalar:
        try:
            sektor_hash[yol.stat().st_size].add(sha256(yol))
        except OSError:
            continue
    # Aynı hash farklı sektörlerde kaç kez geçiyor?
    hash_sektor: dict[str, set[str]] = defaultdict(set)
    for yol, sektor, _video in dosyalar:
        try:
            hash_sektor[sha256(yol)].add(sektor)
        except OSError:
            continue
    capraz = sum(len(s) - 1 for s in hash_sektor.values() if len(s) > 1)

    foto_sayisi = sum(
        1 for yol, _sektor, _neden, _boyut in tekrarlar if yol.suffix.lower() in FOTO_UZANTI
    )
    video_sayisi = len(tekrarlar) - foto_sayisi
    bayt = sum(oge[3] for oge in tekrarlar)
    print(
        f"[TEKİLLE] sektör içi {len(tekrarlar)} tekrar: {birebir} birebir · "
        f"{len(tekrarlar) - birebir} benzer ({foto_sayisi} foto · {video_sayisi} video) · "
        f"{bayt / (1024 * 1024):.1f} MB"
    )
    print(f"[TEKİLLE] sektörler arası birebir kopya: {capraz} (taşınmadı · sektörler boş kalmasın)")
    en_cok = Counter(sektor for _, sektor, _, _ in tekrarlar).most_common(10)
    if en_cok:
        print("[TEKİLLE] en çok tekrar: " + ", ".join(f"{ad}({sayi})" for ad, sayi in en_cok))
    for yol, _sektor, neden, boyut in tekrarlar[:20]:
        print(f"  {yol.relative_to(STOK)} · {boyut / 1024:.0f} KB · {neden}")
    if len(tekrarlar) > 20:
        print(f"  … {len(tekrarlar) - 20} kayıt daha")

    if not secim.uygula:
        print("[TEKİLLE] kuru çalışma · uygulamak için: python medya_temizle.py --uygula")
        return 0

    tasinan = 0
    for yol, _sektor, _neden, _boyut in tekrarlar:
        hedef = TEKRAR / yol.relative_to(STOK)
        hedef.parent.mkdir(parents=True, exist_ok=True)
        try:
            shutil.move(str(yol), str(hedef))
            tasinan += 1
        except OSError as hata:
            print(f"  ! taşınamadı {yol}: {hata}")
    print(f"[TEKİLLE] {tasinan} dosya havuzdan çıkarıldı → {TEKRAR.relative_to(KOK)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
