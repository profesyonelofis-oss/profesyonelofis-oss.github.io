"""Klip üretimini tek komutta arka planda başlatır, ilerlemeyi satır satır gösterir.

Üretim ayrı bir süreçte döner (ffmpeg çocuk süreçleriyle birlikte); bu betik
çıktısını geldiği anda bastığı için uzun dönüşümlerde ekran donmaz, ilerleme
sayaç ve geçen süreyle görünür. Tüm çıktı ayrıca günlük dosyasına yazılır.

Kullanım:
    python araclar/klip_izle.py cekimler/ --yayinla
    python araclar/klip_izle.py cekimler/ --sure 6 --crf 32
    python araclar/klip_izle.py cekim.mp4 --ad kaza

Tüm seçenekler `klip_hazirla.py`'ye aynen geçer (--hedef, --yayinla, --zorla,
--baglama-yok, --kuru ...). Ek seçenek:
    --log YOL   günlük dosyası (varsayılan: _gecici/klip_uretim.log)
"""
import argparse
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import klip_hazirla as kh  # noqa: E402  (yol düzeltmesinden sonra)

BETIK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "klip_hazirla.py")
VARSAYILAN_LOG = os.path.join("_gecici", "klip_uretim.log")
# üretim sürecinin bir çekimi bitirdiğini gösteren satır imleri
TAMAM_IMLERI = ("] çıktı:", "] atlandı:", "] HATA:")


def sure_yazi(saniye: float) -> str:
    return f"{int(saniye) // 60:02d}:{int(saniye) % 60:02d}"


def cekim_sayisi(girdi: str) -> int:
    if girdi and os.path.isdir(girdi):
        return sum(1 for f in os.listdir(girdi) if f.lower().endswith(kh.VIDEO_UZANTI))
    return 1 if girdi else 0


def main() -> int:
    p = argparse.ArgumentParser(
        description="Klip üretimini arka planda çalıştırır, ilerlemeyi satır satır gösterir.",
        epilog="Diğer tüm seçenekler klip_hazirla.py'ye geçirilir.",
    )
    p.add_argument("--log", default=VARSAYILAN_LOG,
                   help=f"günlük dosyası (varsayılan: {VARSAYILAN_LOG})")
    ayar, cocuk = p.parse_known_args()
    if not cocuk:
        p.error("en az bir girdi yolu gerekli (ör. cekimler/)")

    girdi = next((a for a in cocuk if os.path.exists(a)), None)
    toplam = cekim_sayisi(girdi)
    os.makedirs(os.path.dirname(ayar.log) or ".", exist_ok=True)

    ortam = dict(os.environ, PYTHONUNBUFFERED="1", PYTHONIOENCODING="utf-8")
    baslangic = time.time()
    bitti = 0
    print(f"klip üretimi başladı · {toplam or '?'} çekim · günlük: {ayar.log}", flush=True)

    with open(ayar.log, "w", encoding="utf-8") as gunluk:
        proc = subprocess.Popen(
            [sys.executable, "-u", BETIK] + cocuk,
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, encoding="utf-8", errors="replace", bufsize=1, env=ortam,
        )
        try:
            for satir in proc.stdout:
                satir = satir.rstrip("\n")
                gunluk.write(satir + "\n")
                gunluk.flush()
                if any(im in satir for im in TAMAM_IMLERI):
                    bitti += 1
                sayac = f"{min(bitti, toplam)}/{toplam}" if toplam else str(bitti)
                print(f"[{sure_yazi(time.time() - baslangic)} {sayac}] {satir}", flush=True)
        except KeyboardInterrupt:
            proc.terminate()
            print("\nkesildi — üretim durduruldu (yarım kalan klip yeniden üretilir)", flush=True)
            return 130
        kod = proc.wait()

    print(f"\nbitti · çıkış kodu {kod} · süre {sure_yazi(time.time() - baslangic)}"
          f" · günlük: {ayar.log}", flush=True)
    return kod


if __name__ == "__main__":
    raise SystemExit(main())
