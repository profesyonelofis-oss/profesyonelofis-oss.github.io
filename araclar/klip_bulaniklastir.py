"""Kendi çekimlerinizde plaka / telefon gibi bölgeleri mozaikler.

Bu araç yalnızca elinizdeki dosyalar üzerinde çalışır (YouTube'dan indirme yapmaz).
Girdi: telefon/kamera ile çekilmiş mp4/mov. Çıktı: 9:16 dikey, H.264, web'e hazır klip.

Kullanım:
  python araclar/klip_bulaniklastir.py klipler/giris.mp4 --kareler
  python araclar/klip_bulaniklastir.py klipler/giris.mp4 --dikey \
      --bolge 0.30,0.60,0.40,0.12 --bolge 0.10,0.86,0.55,0.08
  python araclar/klip_bulaniklastir.py klipler/giris.mp4 --dikey \
      --bolge 0.30,0.60,0.40,0.12 --cikis klipler/plaka-gizli.mp4

--bolge x,y,w,h : 0-1 arası oran (sol, üst, genişlik, yükseklik). Birden çok kez verilebilir.
--kareler       : mozaiklemeden ÖNCE 4 kontrol karesi çıkarır (bölgeyi doğrulamak için).
--blok          : mozaik kare boyu; küçük değer = daha ince mozaik (varsayılan 14).
--pay           : bölgeyi kenarlardan %N genişletir (varsayılan 6; taşan yazı için).
"""

import argparse
import json
import os
import subprocess
import sys

FFMPEG = "ffmpeg"
FFPROBE = "ffprobe"


def calistir(*args: str) -> None:
    sonuc = subprocess.run([FFMPEG, "-y", "-loglevel", "error", *args])
    if sonuc.returncode != 0:
        raise SystemExit("ffmpeg hata verdi")


def boyut(yol: str) -> tuple[int, int]:
    cikti = subprocess.run(
        [FFPROBE, "-v", "error", "-select_streams", "v:0",
         "-show_entries", "stream=width,height", "-of", "json", yol],
        capture_output=True, text=True, check=True).stdout
    s = json.loads(cikti)["streams"][0]
    return int(s["width"]), int(s["height"])


def bolge_oku(metin: str) -> tuple[float, float, float, float]:
    parca = [float(p) for p in metin.replace(" ", "").split(",")]
    if len(parca) != 4:
        raise SystemExit("--bolge x,y,w,h biçiminde olmalı: " + metin)
    x, y, w, h = parca
    if not all(0 <= v <= 1 for v in parca) or w <= 0 or h <= 0:
        raise SystemExit("--bolge değerleri 0-1 arasında olmalı: " + metin)
    return x, y, w, h


def main() -> int:
    ap = argparse.ArgumentParser(description="Plaka/telefon bölgelerini mozaikler.")
    ap.add_argument("giris")
    ap.add_argument("--cikis")
    ap.add_argument("--bolge", action="append", default=[])
    ap.add_argument("--dikey", action="store_true", help="9:16 (1080x1920) kırpar")
    ap.add_argument("--blok", type=int, default=14)
    ap.add_argument("--pay", type=float, default=6)
    ap.add_argument("--kareler", action="store_true")
    a = ap.parse_args()

    if not os.path.exists(a.giris):
        raise SystemExit("giriş bulunamadı: " + a.giris)

    kok, _ = os.path.splitext(a.giris)
    if a.kareler:
        os.makedirs("_gecici", exist_ok=True)
        g = boyut(a.giris)
        print(f"giriş {g[0]}x{g[1]}")
        for i, sn in enumerate((0.2, 1.0, 2.0, 3.0), start=1):
            hedef = f"_gecici/kare-{i}.jpg"
            calistir("-ss", str(sn), "-i", a.giris, "-frames:v", "1", "-q:v", "3", hedef)
            print("  kare:", hedef)
        print("bölgeyi bu karelerde ölçüp --bolge ile verin.")
        if not a.bolge:
            return 0

    if not a.bolge:
        raise SystemExit("mozaik için en az bir --bolge gerekli (önce --kareler ile bakın)")

    g = boyut(a.giris)
    if a.dikey:
        W, H = 1080, 1920
        temel = (f"scale={W}:{H}:force_original_aspect_ratio=increase,"
                 f"crop={W}:{H},setsar=1")
    else:
        W, H = g[0] // 2 * 2, g[1] // 2 * 2
        temel = f"scale={W}:{H},setsar=1"

    parcalar = [f"[0:v]{temel}[v0]"]
    for i, metin in enumerate(a.bolge):
        x, y, w, h = bolge_oku(metin)
        p = a.pay / 100.0
        x0, y0 = max(0.0, x - w * p), max(0.0, y - h * p)
        x1, y1 = min(1.0, x + w * (1 + p)), min(1.0, y + h * (1 + p))
        px, py = int(x0 * W) // 2 * 2, int(y0 * H) // 2 * 2
        pw = max(2, (int(x1 * W) - px) // 2 * 2)
        ph = max(2, (int(y1 * H) - py) // 2 * 2)
        parcalar.append(f"[v{i}]split[va{i}][vb{i}]")
        kucuk_w, kucuk_h = max(2, pw // a.blok), max(2, ph // a.blok)
        parcalar.append(f"[vb{i}]crop={pw}:{ph}:{px}:{py},"
                        f"scale={kucuk_w}:{kucuk_h}:flags=neighbor,"
                        f"scale={pw}:{ph}:flags=neighbor[vc{i}]")
        parcalar.append(f"[va{i}][vc{i}]overlay={px}:{py}[v{i + 1}]")
        print(f"  mozaik {i + 1}: {pw}x{ph} @ {px},{py}")

    filtre = ";".join(parcalar)
    cikis = a.cikis or f"{kok}-plaka-gizli.mp4"
    os.makedirs(os.path.dirname(cikis) or ".", exist_ok=True)
    calistir("-i", a.giris, "-filter_complex", filtre, "-map", f"[v{len(a.bolge)}]",
             "-map", "0:a?", "-c:v", "libx264", "-crf", "24", "-preset", "veryfast",
             "-pix_fmt", "yuv420p", "-movflags", "+faststart",
             "-c:a", "aac", "-b:a", "96k", cikis)
    kapak = os.path.splitext(cikis)[0] + ".jpg"
    calistir("-ss", "0.5", "-i", cikis, "-frames:v", "1", "-q:v", "4", kapak)
    print("çıktı:", cikis, "· kapak:", kapak,
          f"· {os.path.getsize(cikis) / 1024 / 1024:.1f} MB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
