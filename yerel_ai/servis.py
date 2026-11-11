#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Yerel AI fotoğraf üretim servisi (Stable Diffusion 1.5 · CUDA).

Uygulama bu betiği arka planda gizli pencerede başlatır ve yalnız
127.0.0.1 loopback adresinde JSON ile konuşur:

    GET  /saglik  → {"durum": "yukleniyor" | "hazir" | "hata", "mesaj": ...}
    POST /uret    → {"prompt": ..., "cikti": "media/stock/cekici/ai_x.jpg", ...}
    GET  /kapat   → servisi kapatır

Model ağırlıkları ilk açılışta `HF_HOME` (uygulama `yerel_ai/model` verir)
altına indirilir; sonraki çalışmalar tamamen çevrimdışıdır ve RTX 3050'nin
6 GB VRAM'ine sığar (fp16 + dikkat/VAE dilimleme).
"""

import json
import os
import random
import threading
import time
import traceback
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

KOK = os.path.dirname(os.path.abspath(__file__))
PROJE = os.path.dirname(KOK)
STOK = os.path.join(PROJE, "media", "stock")
MODEL = os.environ.get("YEREL_AI_MODEL", "SG161222/Realistic_Vision_V5.1_noVAE")
PORT = int(os.environ.get("YEREL_AI_PORT", "47621"))
NEGATIF = os.environ.get(
    "YEREL_AI_NEGATIF",
    "blurry, low quality, lowres, jpeg artifacts, watermark, text, letters, logo, "
    "signature, deformed wheels, extra wheels, duplicate, cropped, cartoon, "
    "illustration, painting, oversaturated, tanker, tank truck, fuel truck, "
    "cement mixer, box truck, cargo container, graffiti",
)

# Servis durumu: UI bu alanı `/saglik` üzerinden okur.
_DURUM = {
    "durum": "yukleniyor",
    "mesaj": "Model hazırlanıyor…",
    "model": MODEL,
    "cihaz": "cpu",
    "vram": "",
}
_KILIT = threading.Lock()
_BORU = None


def _yaz(metin):
    """Günlük satırı (uygulama çıktıyı `.gecici/yerel_ai_servis.log`'a yazar)."""
    print(metin, flush=True)


def _guncelle(**alanlar):
    with _KILIT:
        _DURUM.update(alanlar)


def _kopya():
    with _KILIT:
        return dict(_DURUM)


def _olcu(deger, varsayilan):
    """Kare ölçüsü: 64'ün katı, 256–1024 arası (SD 1.5 ve 6 GB VRAM için)."""
    try:
        sayi = int(deger)
    except (TypeError, ValueError):
        sayi = varsayilan
    return max(256, min(1024, sayi // 64 * 64))


def _cikti_dogrula(deger):
    """Çıktı yalnız `media/stock/` altına yazılabilir."""
    if not deger:
        raise RuntimeError("Çıktı yolu verilmedi.")
    yol = os.path.abspath(str(deger))
    kok = os.path.abspath(STOK)
    if os.path.commonpath([yol, kok]) != kok:
        raise RuntimeError("Çıktı yolu medya arşivinin dışında.")
    return yol


def _model_yukle():
    """Modeli fp16 + CUDA ile yükler; durum alanını günceller."""
    global _BORU
    bas = time.time()
    try:
        import torch  # noqa: F401  (CUDA bağımlılığı)
        from diffusers import StableDiffusionPipeline

        if torch.cuda.is_available():
            cihaz, tur = "cuda", torch.float16
            torch.backends.cuda.matmul.allow_tf32 = True
            torch.backends.cudnn.allow_tf32 = True
        else:
            cihaz, tur = "cpu", torch.float32
        _guncelle(cihaz=cihaz, mesaj="Ağırlıklar okunuyor…")
        _yaz(f"YEREL_AI model yükleniyor: {MODEL} ({cihaz})")

        boru = StableDiffusionPipeline.from_pretrained(
            MODEL,
            torch_dtype=tur,
            safety_checker=None,
            requires_safety_checker=False,
        )
        boru = boru.to(cihaz)
        boru.set_progress_bar_config(disable=True)
        # 6 GB VRAM: kod çözme ve dikkat katmanları dilimlenir. diffusers
        # sürümleri bu yardımcıları taşıdığı için hem pipeline hem VAE
        # yöntemleri yoklanır; olan çağrılır.
        vae = getattr(boru, "vae", None)
        for ad, cagri in (
            ("vae_slicing", getattr(boru, "enable_vae_slicing", None)),
            ("vae_tiling", getattr(boru, "enable_vae_tiling", None)),
            ("attention_slicing", getattr(boru, "enable_attention_slicing", None)),
            ("vae.enable_slicing", getattr(vae, "enable_slicing", None)),
            ("vae.enable_tiling", getattr(vae, "enable_tiling", None)),
        ):
            if not callable(cagri):
                continue
            try:
                cagri()
            except Exception as hata:  # noqa: BLE001
                _yaz(f"[!] {ad}: {hata}")
        try:
            # PNDM yerine DPM++ 2M Karras: daha az adımda daha temiz doku.
            # `final_sigmas_type` açıkça verilir; aksi hâlde Karras eğrisi
            # depodaki "deis" algoritmasıyla çakışıp varsayılana dönülüyor.
            from diffusers import DPMSolverMultistepScheduler

            boru.scheduler = DPMSolverMultistepScheduler.from_config(
                boru.scheduler.config,
                algorithm_type="dpmsolver++",
                use_karras_sigmas=True,
                final_sigmas_type="sigma_min",
            )
            _yaz("YEREL_AI scheduler=DPMSolverMultistep(dpmsolver++, karras)")
        except Exception as hata:  # noqa: BLE001
            _yaz(f"[!] scheduler varsayılanda kaldı: {hata}")

        _BORU = boru
        # Isınma karesi: ilk gerçek istek derleme gecikmesi yaşamasın.
        try:
            boru(
                prompt="warmup",
                num_inference_steps=2,
                guidance_scale=1.0,
                width=384,
                height=384,
            )
        except Exception as hata:  # noqa: BLE001
            _yaz(f"[!] ısınma atlandı: {hata}")

        vram = ""
        if cihaz == "cuda":
            bos, toplam = torch.cuda.mem_get_info()
            vram = f"{bos // (1024**2)} / {toplam // (1024**2)} MB boş"
        sure = time.time() - bas
        _guncelle(
            durum="hazir",
            mesaj=f"Model hazır · {sure:.0f} sn'de yüklendi",
            cihaz=cihaz,
            vram=vram,
        )
        _yaz(f"YEREL_AI hazir cihaz={cihaz} sure={sure:.1f}s")
    except Exception as hata:  # noqa: BLE001
        ayrinti = f"{type(hata).__name__}: {hata}"
        _yaz("[!] model yüklenemedi:\n" + traceback.format_exc())
        _guncelle(
            durum="hata",
            mesaj=(
                "Model yüklenemedi. Kurulumu kontrol edin: python -m pip install "
                "torch diffusers transformers accelerate. "
                f"Ayrıntı: {ayrinti}"
            ),
        )


def _uret(istek):
    """Tek bir kare üretir ve JPEG olarak istenen yola yazar."""
    if _BORU is None:
        raise RuntimeError("Model henüz hazır değil.")
    istem = str(istek.get("prompt") or "").strip()
    if not istem:
        raise RuntimeError("Prompt boş olamaz.")
    genislik = _olcu(istek.get("genislik"), 768)
    yukseklik = _olcu(istek.get("yukseklik"), 512)
    try:
        adim = max(4, min(60, int(istek.get("adim") or 28)))
    except (TypeError, ValueError):
        adim = 28
    try:
        rehber = max(0.0, min(16.0, float(istek.get("rehber") or 6.5)))
    except (TypeError, ValueError):
        rehber = 6.5
    try:
        tohum = int(istek.get("tohum", -1))
    except (TypeError, ValueError):
        tohum = -1
    if tohum < 0:
        tohum = random.randint(0, 2**31 - 1)
    cikti = _cikti_dogrula(istek.get("cikti"))
    negatif = str(istek.get("negatif") or NEGATIF)

    import torch

    cihaz = _kopya().get("cihaz", "cuda")
    with _KILIT:
        uretici = torch.Generator(device=cihaz).manual_seed(tohum)
        bas = time.time()
        kare = _BORU(
            prompt=istem,
            negative_prompt=negatif,
            width=genislik,
            height=yukseklik,
            num_inference_steps=adim,
            guidance_scale=rehber,
            generator=uretici,
        ).images[0]
        gecen = time.time() - bas

    os.makedirs(os.path.dirname(cikti), exist_ok=True)
    kare.save(cikti, "JPEG", quality=92, optimize=True, progressive=True)
    _yaz(f"YEREL_AI uretildi {os.path.basename(cikti)} {gecen:.1f}s tohum={tohum}")
    return {
        "ok": True,
        "yol": cikti,
        "sure": round(gecen, 2),
        "tohum": tohum,
        "genislik": kare.width,
        "yukseklik": kare.height,
    }


class Istek(BaseHTTPRequestHandler):
    """Loopback JSON köprüsü; tarayıcı erişimi yoktur."""

    server_version = "YerelAI/1.0"

    def log_message(self, bicim, *arg):  # noqa: N802 (std adı)
        _yaz("%s %s" % (self.address_string(), bicim % arg))

    def _json(self, kod, govde):
        veri = json.dumps(govde, ensure_ascii=False).encode("utf-8")
        self.send_response(kod)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(veri)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(veri)

    def do_GET(self):  # noqa: N802
        if self.path.startswith("/saglik"):
            self._json(200, _kopya())
        elif self.path.startswith("/kapat"):
            self._json(200, {"ok": True})
            threading.Thread(target=self.server.shutdown, daemon=True).start()
        else:
            self._json(404, {"ok": False, "hata": "bilinmeyen yol"})

    def do_POST(self):  # noqa: N802
        if not self.path.startswith("/uret"):
            self._json(404, {"ok": False, "hata": "bilinmeyen yol"})
            return
        try:
            uzunluk = int(self.headers.get("Content-Length") or 0)
            istek = json.loads(self.rfile.read(uzunluk) or b"{}")
            self._json(200, _uret(istek))
        except RuntimeError as hata:
            self._json(400, {"ok": False, "hata": str(hata)})
        except Exception as hata:  # noqa: BLE001
            _yaz("[!] üretim hatası:\n" + traceback.format_exc())
            self._json(500, {"ok": False, "hata": f"{type(hata).__name__}: {hata}"})


def main():
    threading.Thread(target=_model_yukle, name="model", daemon=True).start()
    sunucu = ThreadingHTTPServer(("127.0.0.1", PORT), Istek)
    sunucu.daemon_threads = True
    _yaz(f"YEREL_AI dinleniyor port={PORT} model={MODEL} hf_home={os.environ.get('HF_HOME', '')}")
    try:
        sunucu.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        sunucu.server_close()


if __name__ == "__main__":
    main()
