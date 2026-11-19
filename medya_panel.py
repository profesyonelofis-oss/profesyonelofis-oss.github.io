# -*- coding: utf-8 -*-
"""Sektör ve Medya Yönetim Paneli · yerel web arayüzü.

Çalıştırma:
    python medya_panel.py            # http://127.0.0.1:5000
    python medya_panel.py --port 5050

Arşiv kökü varsayılan `media/stock`; `MEDYA_STOK` ortam değişkeniyle değişir.
Yalnızca Flask gerekir; .exe veya derleme yok.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
from pathlib import Path
from urllib.parse import quote

from flask import Flask, abort, jsonify, render_template_string, request, send_file

VIDEO_UZANTI = {".mp4", ".webm", ".mov", ".mkv", ".m4v"}
FOTO_UZANTI = {".webp", ".jpg", ".jpeg", ".png", ".gif", ".avif"}
KOK = Path(__file__).resolve().parent
STOK = Path(os.environ.get("MEDYA_STOK", KOK / "media" / "stock")).resolve()

app = Flask(__name__)
app.config["MAX_CONTENT_LENGTH"] = 512 * 1024 * 1024  # tek istekte 512 MB


# --------------------------------------------------------------------------- #
# Yardımcılar
# --------------------------------------------------------------------------- #
def slug(ad: str) -> str:
    """Türkçe adı güvenli klasör adına çevirir: 'Bahçe Merkezi' → 'bahce_merkezi'."""
    esleme = str.maketrans(
        {
            "ç": "c",
            "ğ": "g",
            "ı": "i",
            "ö": "o",
            "ş": "s",
            "ü": "u",
            "Ç": "c",
            "Ğ": "g",
            "İ": "i",
            "Ö": "o",
            "Ş": "s",
            "Ü": "u",
        }
    )
    metin = ad.translate(esleme).lower()
    metin = re.sub(r"[^a-z0-9]+", "_", metin).strip("_")
    return metin[:60]


def icinde(yol: Path, kok: Path) -> bool:
    """Yol, kökün içinde mi? (dizin atlama koruması)"""
    try:
        yol.resolve().relative_to(kok.resolve())
        return True
    except ValueError:
        return False


def sektor_yolu(ad: str) -> Path:
    """Doğrulanmış sektör klasörü; yoksa 404."""
    temiz = ad if re.fullmatch(r"[a-z0-9_\-]{1,60}", ad) else slug(ad)
    if not re.fullmatch(r"[a-z0-9_\-]{1,60}", temiz):
        abort(400, "geçersiz sektör adı")
    yol = (STOK / temiz).resolve()
    if not icinde(yol, STOK) or not yol.is_dir():
        abort(404, "sektör bulunamadı")
    return yol


def medya_dosyalari(sektor: Path) -> list[dict]:
    """Sektördeki video ve fotoğraflar (video/ ve photos/ dahil)."""
    kayitlar: list[dict] = []
    for alt in sorted(sektor.iterdir()):
        if alt.name.startswith("."):
            continue
        if alt.is_file():
            dosyalar = [alt]
            grup = "kök"
        elif alt.is_dir() and not alt.is_symlink():
            dosyalar = [d for d in sorted(alt.iterdir()) if d.is_file()]
            grup = alt.name
        else:
            continue
        for dosya in dosyalar:
            uzanti = dosya.suffix.lower()
            if uzanti in VIDEO_UZANTI:
                tip = "video"
            elif uzanti in FOTO_UZANTI:
                tip = "foto"
            else:
                continue
            goreli = dosya.relative_to(sektor).as_posix()
            poster = None
            if tip == "video":
                for aday in (".webp", ".jpg", ".jpeg", ".png"):
                    yan = dosya.with_suffix(aday)
                    if yan.is_file():
                        poster = f"/medya/{quote(sektor.name)}/{quote(yan.relative_to(sektor).as_posix())}"
                        break
            kayitlar.append(
                {
                    "ad": dosya.name,
                    "yol": goreli,
                    "tip": tip,
                    "grup": grup,
                    "boyut": dosya.stat().st_size,
                    "url": f"/medya/{quote(sektor.name)}/{quote(goreli)}",
                    "poster": poster,
                }
            )
    kayitlar.sort(key=lambda k: (k["tip"] != "foto", k["ad"]))
    return kayitlar


def sektor_ozeti(sektor: Path) -> dict:
    medya = medya_dosyalari(sektor)
    video = sum(1 for m in medya if m["tip"] == "video")
    foto = len(medya) - video
    return {
        "ad": sektor.name,
        "video": video,
        "foto": foto,
        "toplam": len(medya),
        "mb": round(sum(m["boyut"] for m in medya) / (1024 * 1024), 2),
    }


def sonraki_ad(klasor: Path, onek: str, uzanti: str) -> Path:
    """hero_3.mp4 / img_12.webp gibi ilk boş numarayı bulur."""
    kullanilan = set()
    if klasor.is_dir():
        for dosya in klasor.iterdir():
            e = re.fullmatch(rf"{onek}_(\d+)\.[A-Za-z0-9]+", dosya.name)
            if e:
                kullanilan.add(int(e.group(1)))
    sira = 1
    while sira in kullanilan:
        sira += 1
    return klasor / f"{onek}_{sira}{uzanti}"


def dosya_sil(sektor: Path, hedef: Path) -> int:
    """Dosyayı ve varsa video afişini siler; silinen bayt sayısını döndürür."""
    bayt = hedef.stat().st_size
    hedef.unlink()
    for aday in (".webp", ".jpg", ".jpeg", ".png"):
        yan = hedef.with_suffix(aday)
        if yan.is_file() and icinde(yan, sektor):
            bayt += yan.stat().st_size
            yan.unlink()
    return bayt


# --------------------------------------------------------------------------- #
# API
# --------------------------------------------------------------------------- #
@app.get("/api/sektorler")
def api_sektorler():
    if not STOK.is_dir():
        return jsonify({"kok": str(STOK), "sektorler": [], "hata": "arşiv klasörü yok"})
    liste = [
        sektor_ozeti(d)
        for d in sorted(STOK.iterdir())
        if d.is_dir() and not d.name.startswith(".") and not d.is_symlink()
    ]
    return jsonify(
        {
            "kok": str(STOK),
            "sektorler": liste,
            "video": sum(s["video"] for s in liste),
            "foto": sum(s["foto"] for s in liste),
            "mb": round(sum(s["mb"] for s in liste), 1),
        }
    )


@app.post("/api/sektorler")
def api_sektor_ekle():
    veri = request.get_json(silent=True) or {}
    ham = str(veri.get("ad", "")).strip()
    if not ham:
        abort(400, "sektör adı boş")
    ad = slug(ham)
    if not ad:
        abort(400, "sektör adı yalnızca harf, rakam ve boşluk içerebilir")
    yol = (STOK / ad).resolve()
    if not icinde(yol, STOK):
        abort(400, "geçersiz yol")
    if yol.exists():
        abort(409, f"'{ad}' klasörü zaten var")
    (yol / "video").mkdir(parents=True)
    (yol / "photos").mkdir()
    return jsonify({"ad": ad, "olustu": True}), 201


@app.delete("/api/sektorler/<ad>")
def api_sektor_sil(ad: str):
    yol = sektor_yolu(ad)
    medya = medya_dosyalari(yol)
    shutil.rmtree(yol)
    return jsonify({"ad": yol.name, "silinen_medya": len(medya)})


@app.post("/api/sektorler/toplu-sil")
def api_sektor_toplu_sil():
    """Seçilen sektörleri klasörleriyle birlikte siler."""
    veri = request.get_json(silent=True) or {}
    adlar = veri.get("sektorler") or []
    if not isinstance(adlar, list) or not adlar:
        abort(400, "sektör seçilmedi")
    silinen, hatalar = [], []
    for ad in adlar:
        try:
            yol = sektor_yolu(str(ad))
            medya = medya_dosyalari(yol)
            shutil.rmtree(yol)
            silinen.append({"ad": yol.name, "medya": len(medya)})
        except Exception as hata:  # HTTPException burada hata olarak raporlanır
            hatalar.append(
                {
                    "ad": str(ad),
                    "hata": getattr(hata, "description", str(hata)),
                }
            )
    return jsonify({"silinen": silinen, "hatalar": hatalar})


@app.get("/api/sektorler/<ad>/medya")
def api_medya(ad: str):
    yol = sektor_yolu(ad)
    medya = medya_dosyalari(yol)
    return jsonify(
        {
            "sektor": yol.name,
            "medya": medya,
            "video": sum(1 for m in medya if m["tip"] == "video"),
            "foto": sum(1 for m in medya if m["tip"] == "foto"),
        }
    )


@app.delete("/api/medya")
def api_medya_sil():
    veri = request.get_json(silent=True) or {}
    sektor = sektor_yolu(str(veri.get("sektor", "")))
    goreli = str(veri.get("yol", ""))
    hedef = (sektor / goreli).resolve()
    if not icinde(hedef, sektor) or not hedef.is_file():
        abort(404, "dosya bulunamadı")
    bayt = dosya_sil(sektor, hedef)
    return jsonify({"silinen": hedef.name, "bayt": bayt})


@app.post("/api/medya/toplu-sil")
def api_medya_toplu_sil():
    """Seçilen medya dosyalarını tek istekte diskten siler."""
    veri = request.get_json(silent=True) or {}
    sektor = sektor_yolu(str(veri.get("sektor", "")))
    yollar = veri.get("yollar") or []
    if not isinstance(yollar, list) or not yollar:
        abort(400, "medya seçilmedi")
    silinen, hatalar, bayt = 0, [], 0
    for goreli in yollar:
        hedef = (sektor / str(goreli)).resolve()
        if not icinde(hedef, sektor) or not hedef.is_file():
            hatalar.append(str(goreli))
            continue
        bayt += dosya_sil(sektor, hedef)
        silinen += 1
    return jsonify({"silinen": silinen, "bayt": bayt, "hatalar": hatalar})


@app.post("/api/medya/yukle")
def api_medya_yukle():
    sektor = sektor_yolu(str(request.form.get("sektor", "")))
    dosyalar = request.files.getlist("dosyalar")
    if not dosyalar:
        abort(400, "dosya seçilmedi")
    kaydedilen, atlanan = [], []
    for gelen in dosyalar:
        ad = Path(gelen.filename or "").name
        uzanti = Path(ad).suffix.lower()
        if uzanti in VIDEO_UZANTI:
            klasor, onek = sektor / "video", "hero"
        elif uzanti in FOTO_UZANTI:
            klasor, onek = sektor / "photos", "img"
        else:
            atlanan.append(ad)
            continue
        klasor.mkdir(exist_ok=True)
        hedef = sonraki_ad(klasor, onek, uzanti)
        gelen.save(hedef)
        kaydedilen.append(hedef.relative_to(sektor).as_posix())
    if not kaydedilen:
        abort(400, f"desteklenmeyen dosya türü: {', '.join(atlanan)}")
    return jsonify({"kaydedilen": kaydedilen, "atlanan": atlanan}), 201


@app.get("/medya/<sektor>/<path:goreli>")
def medya_dosyasi(sektor: str, goreli: str):
    kok = sektor_yolu(sektor)
    hedef = (kok / goreli).resolve()
    if not icinde(hedef, kok) or not hedef.is_file():
        abort(404)
    return send_file(hedef, conditional=True)


# --------------------------------------------------------------------------- #
# Arayüz
# --------------------------------------------------------------------------- #
SAYFA = r"""<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Sektör ve Medya Yönetim Paneli</title>
<style>
  :root{--zemin:#0d1117;--kart:#161b22;--kart2:#1c2330;--cizgi:#26303d;--metin:#e6edf3;
        --soluk:#8b98a8;--vurgu:#3fb950;--mavi:#58a6ff;--kirmizi:#f85149;--turuncu:#d29922}
  *{box-sizing:border-box}
  body{margin:0;background:var(--zemin);color:var(--metin);font:14px/1.45 "Segoe UI",system-ui,sans-serif}
  header{display:flex;flex-wrap:wrap;gap:14px;align-items:center;justify-content:space-between;
         padding:14px 20px;border-bottom:1px solid var(--cizgi);background:#10161f;position:sticky;top:0;z-index:5}
  h1{font-size:16px;margin:0;letter-spacing:.4px}
  h1 span{color:var(--soluk);font-weight:400;font-size:12px;margin-left:8px}
  .ozet{display:flex;gap:16px;font-size:13px;color:var(--soluk);flex-wrap:wrap}
  .ozet b{color:var(--metin);font-variant-numeric:tabular-nums}
  .yerlesim{display:grid;grid-template-columns:340px 1fr;gap:16px;padding:16px 20px 40px}
  @media (max-width:900px){.yerlesim{grid-template-columns:1fr}}
  .kutu{background:var(--kart);border:1px solid var(--cizgi);border-radius:10px}
  .kutu-baslik{display:flex;align-items:center;justify-content:space-between;gap:8px;
               padding:10px 12px;border-bottom:1px solid var(--cizgi);font-size:12px;
               letter-spacing:.6px;text-transform:uppercase;color:var(--soluk)}
  input,select,button{font:inherit;color:var(--metin);background:var(--kart2);
                      border:1px solid var(--cizgi);border-radius:8px;padding:8px 10px}
  input:focus,select:focus{outline:1px solid var(--mavi)}
  button{cursor:pointer;transition:.15s}
  button:hover{border-color:#3d4a5c}
  button:disabled{opacity:.45;cursor:not-allowed}
  button.ana{background:#1f6feb;border-color:#1f6feb;font-weight:600}
  button.ana:hover{background:#2b7cf7}
  button.sil{background:transparent;border-color:transparent;color:var(--kirmizi);padding:4px 8px}
  button.sil:hover{background:#2d1214;border-color:#5a2226}
  button.tehlike{background:#2d1214;border-color:#5a2226;color:#ff9b95;font-size:12px;font-weight:600}
  button.tehlike:hover:not(:disabled){background:#3a171a}
  .kutu-sec{width:16px;height:16px;accent-color:#1f6feb;margin:0;cursor:pointer;flex:none}
  .ekle{display:flex;gap:8px;padding:10px 12px;border-bottom:1px solid var(--cizgi)}
  .ekle input{flex:1;min-width:0}
  .araclar{display:flex;gap:8px;padding:10px 12px;border-bottom:1px solid var(--cizgi);align-items:center}
  .araclar input{flex:1}
  .toplu{display:flex;gap:8px;align-items:center;justify-content:space-between;padding:8px 12px;
         border-bottom:1px solid var(--cizgi);font-size:12px;color:var(--soluk)}
  .toplu label{display:flex;gap:6px;align-items:center;cursor:pointer;white-space:nowrap}
  .liste{max-height:calc(100vh - 260px);overflow:auto;padding:6px}
  .sektor{display:flex;align-items:center;gap:8px;padding:8px 10px;border-radius:8px;cursor:pointer;
          border:1px solid transparent}
  .sektor:hover{background:var(--kart2)}
  .sektor.secili{background:#132030;border-color:#22405f}
  .sektor .ad{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .sektor .sayi{font-size:12px;color:var(--soluk);font-variant-numeric:tabular-nums;white-space:nowrap}
  .sektor .sayi b{color:var(--mavi);font-weight:600}
  .icerik-baslik{display:flex;flex-wrap:wrap;gap:10px;align-items:center;padding:12px 14px;
                 border-bottom:1px solid var(--cizgi)}
  .icerik-baslik h2{font-size:15px;margin:0}
  .rozet{font-size:11px;color:var(--soluk);border:1px solid var(--cizgi);border-radius:999px;padding:2px 8px}
  .yukle{padding:12px 14px;border-bottom:1px solid var(--cizgi)}
  .damla{display:flex;flex-wrap:wrap;gap:10px;align-items:center;justify-content:space-between;
         border:1px dashed #33404f;border-radius:10px;padding:12px;color:var(--soluk)}
  .damla.uzerinde{border-color:var(--vurgu);background:#0f1a13}
  .damla b{color:var(--metin)}
  .medya-araclar{display:flex;gap:10px;align-items:center;justify-content:space-between;
                 padding:10px 14px;border-bottom:1px solid var(--cizgi);font-size:12px;color:var(--soluk)}
  .medya-araclar label{display:flex;gap:6px;align-items:center;cursor:pointer;white-space:nowrap}
  .izgara{display:grid;grid-template-columns:repeat(auto-fill,minmax(180px,1fr));gap:12px;padding:14px}
  .medya{background:var(--kart2);border:1px solid var(--cizgi);border-radius:10px;overflow:hidden;
         display:flex;flex-direction:column}
  .medya.secili{border-color:#2f6fbb;box-shadow:0 0 0 1px #2f6fbb inset}
  .onizleme{position:relative;aspect-ratio:16/10;background:#0a0f16;display:flex;align-items:center;justify-content:center}
  .onizleme img,.onizleme video{width:100%;height:100%;object-fit:cover;display:block}
  .onizleme .medya-sec{position:absolute;right:8px;top:8px;z-index:2;background:#0d1117cc;border-radius:4px;padding:2px}
  .onizleme .bos{font-size:11px;color:#4b5867}
  .medya .alt{display:flex;align-items:center;gap:6px;padding:8px 10px}
  .medya .dosya{flex:1;min-width:0;font-size:12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .medya .boyut{font-size:11px;color:var(--soluk);font-variant-numeric:tabular-nums}
  .etiket{position:absolute;left:8px;top:8px;font-size:10px;letter-spacing:.5px;padding:2px 7px;
          border-radius:999px;background:#0d1117cc;border:1px solid var(--cizgi);color:var(--soluk)}
  .etiket.video{color:#d29922;border-color:#4a3a12}
  .bos-durum{padding:40px;text-align:center;color:var(--soluk)}
  .bildirim{position:fixed;right:18px;bottom:18px;display:flex;flex-direction:column;gap:8px;z-index:20}
  .toast{background:#1b2431;border:1px solid var(--cizgi);border-left:3px solid var(--vurgu);
         padding:10px 14px;border-radius:8px;font-size:13px;box-shadow:0 8px 24px #0008;max-width:360px}
  .toast.hata{border-left-color:var(--kirmizi)}
</style>
</head>
<body>
<header>
  <h1>SEKTÖR VE MEDYA YÖNETİM PANELİ <span id="kok">media/stock</span></h1>
  <div class="ozet">
    <span>sektör <b id="o-sektor">0</b></span>
    <span>video <b id="o-video">0</b></span>
    <span>fotoğraf <b id="o-foto">0</b></span>
    <span>disk <b id="o-mb">0</b> MB</span>
  </div>
</header>

<div class="yerlesim">
  <section class="kutu">
    <div class="kutu-baslik">Sektörler <span id="sektor-sayi">0</span></div>
    <div class="ekle">
      <input id="yeni-ad" placeholder="yeni sektör adı (ör. Bahçe Merkezi)" autocomplete="off">
      <button class="ana" id="ekle">Ekle</button>
    </div>
    <div class="araclar">
      <select id="sirala" title="Sıralama">
        <option value="ad">Ada göre</option>
        <option value="medya">Medya sayısına göre</option>
      </select>
      <input id="ara" placeholder="sektör ara…" autocomplete="off">
    </div>
    <div class="toplu">
      <label><input type="checkbox" class="kutu-sec" id="sektor-hepsi"> tümünü seç</label>
      <button class="tehlike" id="sektor-toplu-sil" disabled>Seçilenleri Sil (0)</button>
    </div>
    <div class="liste" id="liste"></div>
  </section>

  <section class="kutu">
    <div class="icerik-baslik">
      <h2 id="baslik">Bir sektör seçin</h2>
      <span class="rozet" id="rozet">—</span>
      <span style="flex:1"></span>
      <button class="sil" id="sektor-sil" hidden>Seçili sektörü sil</button>
    </div>
    <div class="yukle" id="yukle-alani" hidden>
      <div class="damla" id="damla">
        <div><b>Dosya ekle</b> · fotoğraf (webp/jpg/png) veya video (mp4/webm) sürükleyin ya da seçin</div>
        <div style="display:flex;gap:8px;align-items:center">
          <input type="file" id="dosyalar" multiple hidden>
          <button class="ana" id="sec">Dosya Seç</button>
        </div>
      </div>
      <div id="yukleme" style="margin-top:8px;font-size:12px;color:var(--soluk)"></div>
    </div>
    <div class="medya-araclar" id="medya-araclar" hidden>
      <label><input type="checkbox" class="kutu-sec" id="medya-hepsi"> tümünü seç</label>
      <button class="tehlike" id="medya-toplu-sil" disabled>Seçilenleri Toplu Sil (0)</button>
    </div>
    <div class="izgara" id="medya"></div>
    <div class="bos-durum" id="bos-durum">Soldan bir sektör seçin; fotoğraf ve videolar burada kartlar hâlinde görünür.</div>
  </section>
</div>
<div class="bildirim" id="bildirim"></div>

<script>
const D = { sektorler: [], sektor: null, medya: [], sirala: "ad", ara: "", secSektor: new Set(), secMedya: new Set() };
const $ = (id) => document.getElementById(id);
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));
const boyut = (b) => b > 1048576 ? (b/1048576).toFixed(1) + " MB" : Math.max(1, Math.round(b/1024)) + " KB";

function bildir(metin, hata) {
  const k = document.createElement("div");
  k.className = "toast" + (hata ? " hata" : "");
  k.textContent = metin;
  $("bildirim").appendChild(k);
  setTimeout(() => k.remove(), 4200);
}

async function istek(yol, secenek = {}) {
  const y = await fetch(yol, secenek);
  if (!y.ok) {
    let mesaj = y.status + " " + y.statusText;
    try { const v = await y.json(); if (v.hata) mesaj = v.hata; } catch (e) {}
    throw new Error(mesaj);
  }
  const tip = y.headers.get("content-type") || "";
  return tip.includes("json") ? y.json() : y.text();
}

async function sektorleriYukle() {
  const v = await istek("/api/sektorler");
  D.sektorler = v.sektorler || [];
  $("o-sektor").textContent = D.sektorler.length;
  $("o-video").textContent = v.video || 0;
  $("o-foto").textContent = v.foto || 0;
  $("o-mb").textContent = v.mb || 0;
  $("kok").textContent = v.kok || "media/stock";
  $("sektor-sayi").textContent = D.sektorler.length;
  cizListe();
}

function gorunenSektorler() {
  const ara = D.ara.toLocaleLowerCase("tr");
  return D.sektorler.filter((s) => s.ad.toLocaleLowerCase("tr").includes(ara));
}

function cizListe() {
  const adlar = new Set(D.sektorler.map((s) => s.ad));
  [...D.secSektor].forEach((a) => { if (!adlar.has(a)) D.secSektor.delete(a); });
  let liste = gorunenSektorler();
  liste = [...liste].sort((a, b) => D.sirala === "medya"
    ? (b.toplam - a.toplam) || a.ad.localeCompare(b.ad, "tr")
    : a.ad.localeCompare(b.ad, "tr"));
  $("liste").innerHTML = liste.length ? liste.map((s) => {
    const secili = s.ad === D.sektor ? " secili" : "";
    const isaret = D.secSektor.has(s.ad) ? " checked" : "";
    return `<div class="sektor${secili}" data-ad="${esc(s.ad)}" title="${esc(s.ad)} · ${s.mb} MB">
      <input type="checkbox" class="kutu-sec" data-sec-sektor="${esc(s.ad)}"${isaret} title="seç">
      <span class="ad">${esc(s.ad)}</span>
      <span class="sayi"><b>${s.foto}</b> foto · <b>${s.video}</b> video</span>
      <button class="sil" data-sil="${esc(s.ad)}" title="Sektörü sil">✕</button>
    </div>`;
  }).join("") : `<div class="bos-durum">eşleşen sektör yok</div>`;
  topluDurum();
}

function topluDurum() {
  const n = D.secSektor.size;
  const dugme = $("sektor-toplu-sil");
  dugme.textContent = `Seçilenleri Sil (${n})`;
  dugme.disabled = n === 0;
  const gorunen = gorunenSektorler();
  $("sektor-hepsi").checked = gorunen.length > 0 && gorunen.every((s) => D.secSektor.has(s.ad));
  $("sektor-hepsi").indeterminate = !$("sektor-hepsi").checked && gorunen.some((s) => D.secSektor.has(s.ad));

  const m = D.secMedya.size;
  $("medya-araclar").hidden = !D.sektor;
  $("medya-toplu-sil").textContent = `Seçilenleri Toplu Sil (${m})`;
  $("medya-toplu-sil").disabled = m === 0;
  $("medya-hepsi").checked = D.medya.length > 0 && D.medya.every((x) => D.secMedya.has(x.yol));
  $("medya-hepsi").indeterminate = !$("medya-hepsi").checked && D.medya.some((x) => D.secMedya.has(x.yol));
}

function panelSifirla() {
  D.sektor = null;
  D.medya = [];
  D.secMedya.clear();
  cizMedya();
  $("baslik").textContent = "Bir sektör seçin";
  $("rozet").textContent = "—";
  $("sektor-sil").hidden = true;
  $("yukle-alani").hidden = true;
  $("bos-durum").hidden = false;
  topluDurum();
}

async function sektorSec(ad) {
  D.sektor = ad;
  D.secMedya.clear();
  cizListe();
  $("baslik").textContent = ad;
  $("sektor-sil").hidden = false;
  $("yukle-alani").hidden = false;
  await medyaYukle();
}

async function medyaYukle() {
  if (!D.sektor) return;
  const v = await istek("/api/sektorler/" + encodeURIComponent(D.sektor) + "/medya");
  D.medya = v.medya || [];
  const gecerli = new Set(D.medya.map((m) => m.yol));
  [...D.secMedya].forEach((y) => { if (!gecerli.has(y)) D.secMedya.delete(y); });
  $("rozet").textContent = `${v.foto} foto · ${v.video} video`;
  cizMedya();
}

function cizMedya() {
  $("bos-durum").hidden = D.medya.length > 0 || !!D.sektor;
  $("medya").innerHTML = D.medya.map((m) => {
    const onizleme = m.tip === "video"
      ? `<video src="${esc(m.url)}" ${m.poster ? `poster="${esc(m.poster)}"` : ""} preload="metadata" muted playsinline controls></video>`
      : `<img src="${esc(m.url)}" alt="${esc(m.ad)}" loading="lazy">`;
    const isaret = D.secMedya.has(m.yol) ? " checked" : "";
    return `<article class="medya${isaret ? " secili" : ""}" data-yol="${esc(m.yol)}">
      <div class="onizleme">
        <input type="checkbox" class="kutu-sec medya-sec" data-sec-medya="${esc(m.yol)}"${isaret} title="seç">
        ${onizleme}<span class="etiket ${m.tip}">${m.tip === "video" ? "VİDEO" : "FOTO"}</span>
      </div>
      <div class="alt">
        <span class="dosya" title="${esc(m.yol)}">${esc(m.ad)}</span>
        <span class="boyut">${boyut(m.boyut)}</span>
        <button class="sil" data-medya-sil="${esc(m.yol)}" title="Dosyayı sil">🗑</button>
      </div>
    </article>`;
  }).join("");
  topluDurum();
}

$("liste").addEventListener("click", async (olay) => {
  const sec = olay.target.closest("[data-sec-sektor]");
  if (sec) {
    const ad = sec.dataset.secSektor;
    if (sec.checked) D.secSektor.add(ad); else D.secSektor.delete(ad);
    topluDurum();
    return;
  }
  const sil = olay.target.closest("[data-sil]");
  if (sil) {
    olay.stopPropagation();
    const ad = sil.dataset.sil;
    const s = D.sektorler.find((x) => x.ad === ad);
    if (!confirm(`"${ad}" sektörü ve içindeki ${s ? s.toplam : 0} medya kalıcı olarak silinsin mi?`)) return;
    try {
      await istek("/api/sektorler/" + encodeURIComponent(ad), { method: "DELETE" });
      D.secSektor.delete(ad);
      bildir(`sektör silindi: ${ad}`);
      if (D.sektor === ad) panelSifirla();
      await sektorleriYukle();
    } catch (h) { bildir("silinemedi: " + h.message, true); }
    return;
  }
  const kutu = olay.target.closest(".sektor");
  if (kutu) await sektorSec(kutu.dataset.ad);
});

$("sektor-hepsi").addEventListener("change", (o) => {
  gorunenSektorler().forEach((s) => {
    if (o.target.checked) D.secSektor.add(s.ad); else D.secSektor.delete(s.ad);
  });
  cizListe();
});

$("sektor-toplu-sil").addEventListener("click", async () => {
  const adlar = [...D.secSektor];
  if (!adlar.length) return;
  const toplam = D.sektorler.filter((s) => adlar.includes(s.ad)).reduce((t, s) => t + s.toplam, 0);
  if (!confirm(`${adlar.length} sektör ve içindeki ${toplam} medya kalıcı olarak silinsin mi?\n\n${adlar.slice(0, 8).join(", ")}${adlar.length > 8 ? " …" : ""}`)) return;
  try {
    const v = await istek("/api/sektorler/toplu-sil", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ sektorler: adlar })
    });
    D.secSektor.clear();
    const secilen = D.sektor;
    bildir(`${v.silinen.length} sektör silindi` + (v.hatalar.length ? ` · ${v.hatalar.length} hata` : ""));
    await sektorleriYukle();
    if (secilen && !D.sektorler.some((s) => s.ad === secilen)) panelSifirla();
    else if (D.sektor) await medyaYukle();
  } catch (h) { bildir("toplu silme başarısız: " + h.message, true); }
});

$("medya").addEventListener("click", async (olay) => {
  const sec = olay.target.closest("[data-sec-medya]");
  if (sec) {
    const yol = sec.dataset.secMedya;
    if (sec.checked) D.secMedya.add(yol); else D.secMedya.delete(yol);
    sec.closest(".medya").classList.toggle("secili", sec.checked);
    topluDurum();
    return;
  }
  const dugme = olay.target.closest("[data-medya-sil]");
  if (!dugme) return;
  const yol = dugme.dataset.medyaSil;
  if (!confirm(`"${yol}" dosyası diskten silinsin mi?`)) return;
  try {
    const v = await istek("/api/medya", {
      method: "DELETE",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ sektor: D.sektor, yol })
    });
    D.secMedya.delete(yol);
    bildir(`${v.silinen} silindi (${boyut(v.bayt)})`);
    await medyaYukle();
    await sektorleriYukle();
  } catch (h) { bildir("silinemedi: " + h.message, true); }
});

$("medya-hepsi").addEventListener("change", (o) => {
  D.medya.forEach((m) => {
    if (o.target.checked) D.secMedya.add(m.yol); else D.secMedya.delete(m.yol);
  });
  cizMedya();
});

$("medya-toplu-sil").addEventListener("click", async () => {
  const yollar = [...D.secMedya];
  if (!yollar.length || !D.sektor) return;
  if (!confirm(`${yollar.length} dosya diskten kalıcı olarak silinsin mi?\n\n${yollar.slice(0, 6).join(", ")}${yollar.length > 6 ? " …" : ""}`)) return;
  try {
    const v = await istek("/api/medya/toplu-sil", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ sektor: D.sektor, yollar })
    });
    D.secMedya.clear();
    bildir(`${v.silinen} dosya silindi (${boyut(v.bayt)})` + (v.hatalar.length ? ` · ${v.hatalar.length} bulunamadı` : ""));
    await medyaYukle();
    await sektorleriYukle();
  } catch (h) { bildir("toplu silme başarısız: " + h.message, true); }
});

$("sektor-sil").addEventListener("click", async () => {
  if (!D.sektor) return;
  const s = D.sektorler.find((x) => x.ad === D.sektor);
  if (!confirm(`"${D.sektor}" sektörü ve içindeki ${s ? s.toplam : 0} medya kalıcı olarak silinsin mi?`)) return;
  const ad = D.sektor;
  try {
    await istek("/api/sektorler/" + encodeURIComponent(ad), { method: "DELETE" });
    D.secSektor.delete(ad);
    bildir(`sektör silindi: ${ad}`);
    panelSifirla();
    await sektorleriYukle();
  } catch (h) { bildir("silinemedi: " + h.message, true); }
});

$("ekle").addEventListener("click", async () => {
  const ad = $("yeni-ad").value.trim();
  if (!ad) { bildir("sektör adı yazın", true); return; }
  try {
    const v = await istek("/api/sektorler", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ad })
    });
    $("yeni-ad").value = "";
    bildir(`sektör oluşturuldu: ${v.ad}`);
    await sektorleriYukle();
    await sektorSec(v.ad);
  } catch (h) { bildir("eklenemedi: " + h.message, true); }
});

$("yeni-ad").addEventListener("keydown", (o) => { if (o.key === "Enter") $("ekle").click(); });
$("sirala").addEventListener("change", (o) => { D.sirala = o.target.value; cizListe(); });
$("ara").addEventListener("input", (o) => { D.ara = o.target.value; cizListe(); });

async function yukle(dosyalar) {
  if (!D.sektor || !dosyalar.length) return;
  const veri = new FormData();
  veri.append("sektor", D.sektor);
  [...dosyalar].forEach((d) => veri.append("dosyalar", d, d.name));
  $("yukleme").textContent = `${dosyalar.length} dosya yükleniyor…`;
  try {
    const v = await istek("/api/medya/yukle", { method: "POST", body: veri });
    $("yukleme").textContent = "";
    bildir(`${v.kaydedilen.length} dosya eklendi` + (v.atlanan.length ? ` · atlandı: ${v.atlanan.join(", ")}` : ""));
    await medyaYukle();
    await sektorleriYukle();
  } catch (h) {
    $("yukleme").textContent = "";
    bildir("yüklenemedi: " + h.message, true);
  }
}

$("sec").addEventListener("click", () => $("dosyalar").click());
$("dosyalar").addEventListener("change", (o) => { yukle(o.target.files); o.target.value = ""; });
const damla = $("damla");
["dragenter", "dragover"].forEach((olay) => damla.addEventListener(olay, (o) => { o.preventDefault(); damla.classList.add("uzerinde"); }));
["dragleave", "drop"].forEach((olay) => damla.addEventListener(olay, (o) => { o.preventDefault(); damla.classList.remove("uzerinde"); }));
damla.addEventListener("drop", (o) => { if (D.sektor) yukle(o.dataTransfer.files); });

sektorleriYukle().catch((h) => bildir("sektörler okunamadı: " + h.message, true));
</script>
</body>
</html>
"""


@app.get("/")
def anasayfa():
    return render_template_string(SAYFA)


def main() -> None:
    ayar = argparse.ArgumentParser(description="Sektör ve Medya Yönetim Paneli")
    ayar.add_argument("--port", type=int, default=5000, help="yerel port (varsayılan 5000)")
    ayar.add_argument("--host", default="127.0.0.1", help="bağlanılacak adres (varsayılan 127.0.0.1)")
    secim = ayar.parse_args()
    if not STOK.is_dir():
        raise SystemExit(f"arşiv klasörü bulunamadı: {STOK}")
    print(f"Sektör ve Medya Yönetim Paneli · arşiv: {STOK}")
    print(f"Tarayıcıda açın: http://{secim.host}:{secim.port}")
    app.run(host=secim.host, port=secim.port, debug=False, use_reloader=False)


if __name__ == "__main__":
    main()
