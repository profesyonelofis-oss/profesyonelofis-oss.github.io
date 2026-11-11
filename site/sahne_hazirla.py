#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Videoları "sahne"lere böler: her sahne kendi kare dizisi + manifest.

Kullanım:
    python site/sahne_hazirla.py                     # site/video içindeki her video = 1 sahne
    python site/sahne_hazirla.py --bol 3             # tek videoyu 3 eşit sahneye böl (demo)
    python site/sahne_hazirla.py --adlar "Yükleme,Motor,Yol"
    python site/sahne_hazirla.py --kare 40 --genislik 1100

Çıktı:
    site/sahneler/01-yukleme/kare_0000.jpg ...
    site/sahneler/manifest.json
"""
import argparse
import datetime
import json
import re
import shutil
import subprocess
import sys
import unicodedata
from pathlib import Path

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass

KOK = Path(__file__).resolve().parent
VIDEO_KLASOR = KOK / "video"
SAHNE_KLASOR = KOK / "sahneler"
UZANTILAR = {".mp4", ".mov", ".mkv", ".webm", ".avi", ".m4v"}


def slug(ad: str) -> str:
    ad = unicodedata.normalize("NFKD", ad).encode("ascii", "ignore").decode()
    ad = re.sub(r"[^a-zA-Z0-9]+", "-", ad).strip("-").lower()
    return ad or "sahne"


def sure_al(video: Path) -> float:
    cikti = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=noprint_wrappers=1:nokey=1", str(video)],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    return float(cikti)


def kareler(video: Path, hedef: Path, bas: float, sure: float, adet: int, genislik: int, kalite: int) -> int:
    hedef.mkdir(parents=True, exist_ok=True)
    fps = adet / sure
    komut = [
        "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
        "-ss", f"{bas:.3f}", "-t", f"{sure:.3f}", "-i", str(video),
        "-vf", f"fps={fps:.6f},scale={genislik}:-2",
        "-q:v", str(kalite), "-start_number", "0",
        str(hedef / "kare_%04d.jpg"),
    ]
    if subprocess.run(komut).returncode != 0:
        return 0
    return len(sorted(hedef.glob("kare_*.jpg")))


def main() -> None:
    ap = argparse.ArgumentParser(description="Videoları sahnelere böl.")
    ap.add_argument("--bol", type=int, default=1, help="tek videoyu kaç sahneye böl (varsayılan 1)")
    ap.add_argument("--kare", type=int, default=40, help="sahne başına kare (varsayılan 40)")
    ap.add_argument("--genislik", type=int, default=1100, help="kare genişliği (varsayılan 1100)")
    ap.add_argument("--kalite", type=int, default=6, help="JPEG kalitesi 2-11 (varsayılan 6)")
    ap.add_argument("--adlar", default="", help="virgülle sahne adları: 'Yükleme,Motor,Yol'")
    args = ap.parse_args()

    if not shutil.which("ffmpeg") or not shutil.which("ffprobe"):
        sys.exit("Hata: ffmpeg bulunamadı. Kurulum: winget install Gyan.FFmpeg")

    videolar = sorted(p for p in VIDEO_KLASOR.glob("*") if p.suffix.lower() in UZANTILAR)
    if not videolar:
        sys.exit(f"Hata: {VIDEO_KLASOR} içinde video yok.")

    adlar = [a.strip() for a in args.adlar.split(",") if a.strip()]

    # eski sahneleri temizle
    if SAHNE_KLASOR.exists():
        for alt in SAHNE_KLASOR.iterdir():
            if alt.is_dir():
                shutil.rmtree(alt)
    SAHNE_KLASOR.mkdir(parents=True, exist_ok=True)

    sahneler = []
    sira = 0
    for video in videolar:
        try:
            toplam = sure_al(video)
        except Exception:
            print(f"Atlandı (süre okunamadı): {video.name}")
            continue
        parca = max(1, args.bol)
        parca_sure = toplam / parca
        for k in range(parca):
            sira += 1
            ad = adlar[sira - 1] if sira - 1 < len(adlar) else video.stem
            klasor = f"{sira:02d}-{slug(ad)}"
            adet = kareler(video, SAHNE_KLASOR / klasor, k * parca_sure, parca_sure,
                           args.kare, args.genislik, args.kalite)
            print(f"{klasor}: {adet} kare ({parca_sure:.1f} sn)")
            if adet:
                sahneler.append({"ad": ad, "klasor": klasor, "kare": adet})

    if not sahneler:
        sys.exit("Hata: hiç sahne üretilemedi.")

    manifest = {
        "sahneler": sahneler,
        "genislik": args.genislik,
        "olusturma": datetime.datetime.now().isoformat(timespec="seconds"),
    }
    (SAHNE_KLASOR / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    toplam_kare = sum(s["kare"] for s in sahneler)
    boyut = sum(f.stat().st_size for f in SAHNE_KLASOR.rglob("*.jpg")) / (1024 * 1024)
    print(f"Tamam: {len(sahneler)} sahne / {toplam_kare} kare / {boyut:.1f} MB -> {SAHNE_KLASOR}")
    print("Sayfayı yenile: tekerlek artık sahneler arasında geçiş yapar.")


if __name__ == "__main__":
    main()
