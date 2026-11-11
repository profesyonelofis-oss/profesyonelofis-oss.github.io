#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""site/video/ klasöründeki videoyu karelere bölüp scroll-scrub için hazırlar.

Kullanım:
    python site/video_scrub.py                 # site/video içindeki ilk video
    python site/video_scrub.py --kare 60       # daha az kare (daha hızlı yükleme)
    python site/video_scrub.py --genislik 1280 # daha küçük kareler
    python site/video_scrub.py --video yol.mp4 # belirli bir dosya

Çıktı:
    site/kareler/kare_0000.jpg ...
    site/kareler/manifest.json   (sayfa bu dosyayı görünce canvas yerine videoyu gösterir)
"""
import argparse
import datetime
import json
import shutil
import subprocess
import sys
from pathlib import Path

try:  # Windows konsolu (cp1254) ok işaretini basamıyor; çıktıyı utf-8'e al
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass

KOK = Path(__file__).resolve().parent
VIDEO_KLASOR = KOK / "video"
KARE_KLASOR = KOK / "kareler"
UZANTILAR = {".mp4", ".mov", ".mkv", ".webm", ".avi", ".m4v"}


def arac(var_mi: str) -> bool:
    return shutil.which(var_mi) is not None


def sure_al(video: Path) -> float:
    cikti = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=noprint_wrappers=1:nokey=1", str(video)],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    return float(cikti)


def video_bul(verilen: str | None) -> Path:
    if verilen:
        yol = Path(verilen)
        if not yol.is_absolute():
            yol = KOK / yol
        if not yol.exists():
            sys.exit(f"Hata: video bulunamadı: {yol}")
        return yol
    if not VIDEO_KLASOR.exists():
        sys.exit(f"Hata: {VIDEO_KLASOR} klasörü yok.")
    adaylar = sorted(
        p for p in VIDEO_KLASOR.iterdir() if p.suffix.lower() in UZANTILAR and p.is_file()
    )
    if not adaylar:
        sys.exit(
            f"Hata: {VIDEO_KLASOR} içinde video yok.\n"
            "Çekici videosunu bu klasöre kopyalayıp komutu tekrar çalıştır."
        )
    return adaylar[0]


def main() -> None:
    ap = argparse.ArgumentParser(description="Videoyu scroll-scrub karelerine böl.")
    ap.add_argument("--video", help="işlenecek video yolu (varsayılan: site/video içindeki ilk video)")
    ap.add_argument("--kare", type=int, default=90, help="çıkarılacak kare sayısı (varsayılan 90)")
    ap.add_argument("--genislik", type=int, default=1600, help="kare genişliği piksel (varsayılan 1600)")
    ap.add_argument("--kalite", type=int, default=4, choices=range(2, 12), metavar="2-11",
                    help="JPEG kalitesi, küçük sayı = daha iyi (varsayılan 4)")
    args = ap.parse_args()

    if not arac("ffmpeg") or not arac("ffprobe"):
        sys.exit(
            "Hata: ffmpeg bulunamadı.\n"
            "Kurulum (yönetici gerekmez): winget install Gyan.FFmpeg\n"
            "Kurduktan sonra terminali kapatıp yeniden aç."
        )

    video = video_bul(args.video)
    try:
        sure = sure_al(video)
    except (subprocess.CalledProcessError, ValueError):
        sys.exit(f"Hata: videonun süresi okunamadı: {video.name}")
    if sure <= 0:
        sys.exit(f"Hata: video süresi sıfır: {video.name}")

    KARE_KLASOR.mkdir(parents=True, exist_ok=True)
    for eski in KARE_KLASOR.glob("kare_*.jpg"):
        eski.unlink()
    manifest = KARE_KLASOR / "manifest.json"
    if manifest.exists():
        manifest.unlink()

    fps = args.kare / sure
    komut = [
        "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
        "-i", str(video),
        "-vf", f"fps={fps:.6f},scale={args.genislik}:-2",
        "-q:v", str(args.kalite),
        "-start_number", "0",
        str(KARE_KLASOR / "kare_%04d.jpg"),
    ]
    print(f"Video   : {video.name}  ({sure:.1f} sn)")
    print(f"Kare    : {args.kare} adet - genislik {args.genislik}px - fps {fps:.4f}")
    sonuc = subprocess.run(komut)
    if sonuc.returncode != 0:
        sys.exit("Hata: ffmpeg kareleri çıkaramadı (yukarıdaki mesaja bak).")

    kareler = sorted(KARE_KLASOR.glob("kare_*.jpg"))
    if not kareler:
        sys.exit("Hata: hiç kare oluşmadı.")

    boyut = sum(k.stat().st_size for k in kareler) / (1024 * 1024)
    veri = {
        "kaynak": video.name,
        "kare": len(kareler),
        "onek": "kare_",
        "genislik": args.genislik,
        "sure": round(sure, 2),
        "boyut_mb": round(boyut, 1),
        "olusturma": datetime.datetime.now().isoformat(timespec="seconds"),
    }
    manifest.write_text(json.dumps(veri, ensure_ascii=False, indent=2), encoding="utf-8")

    print(f"Tamam   : {len(kareler)} kare - {boyut:.1f} MB -> {KARE_KLASOR}")
    print("Sayfayı yenile: hero artık scroll ile bu kareleri oynatır.")


if __name__ == "__main__":
    main()
