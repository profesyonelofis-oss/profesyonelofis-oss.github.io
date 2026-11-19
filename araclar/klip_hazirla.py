"""Telefon çekimini hero klibine çevirir.

Girdi: tek video dosyası **veya** çekimlerin bulunduğu klasör.
Çıktı: 960x540, 24 fps, ses yok, 6 saniye, dikişsiz döngü + poster.

Kırpma akıllıdır: kare örneklerinden "detay enerjisi" profili çıkarılır,
konunun bulunduğu bant aranır. Konu ortadaysa tam merkez kırpılır; değilse
üst-orta bölge (gökyüzü/tavan payı atılmadan) kullanılır.

Kullanım:
    python araclar/klip_hazirla.py cekim.mp4
    python araclar/klip_hazirla.py cekim.mp4 --ad kaza --baslangic 12
    python araclar/klip_hazirla.py cekimler/ --yayinla        # klasör + yayın

Klasör modunda daha önce işlenmiş (kaynağı ve seçenekleri değişmemiş) çekimler
atlanır; durum `_gecici/klip_durum.json` içinde tutulur. `--zorla` ile hepsi
yeniden üretilir.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

HEDEF_KLASOR = os.path.join("site", "video", "hero")
HEDEF_ORAN = 16 / 9
VIDEO_UZANTI = (".mp4", ".mov", ".m4v", ".webm", ".avi", ".mkv")
DURUM_DOSYASI = os.path.join("_gecici", "klip_durum.json")


def _imza(args) -> str:
    """Dönüştürmeyi etkileyen seçeneklerin parmak izi."""
    return json.dumps({
        "baslangic": args.baslangic, "sure": args.sure, "gecis": args.gecis,
        "crf": args.crf, "kirpma": args.kirpma, "klavuzsuz": args.klavuzsuz,
    }, sort_keys=True)


def _anahtar(hedef: str, ad: str) -> str:
    return f"{os.path.normpath(hedef)}|{ad}"


def _durum_oku() -> dict:
    try:
        with open(DURUM_DOSYASI, encoding="utf-8") as f:
            d = json.load(f)
        return d if isinstance(d, dict) else {}
    except (OSError, ValueError):
        return {}


def _durum_yaz(durum: dict) -> None:
    os.makedirs(os.path.dirname(DURUM_DOSYASI) or ".", exist_ok=True)
    with open(DURUM_DOSYASI, "w", encoding="utf-8") as f:
        json.dump(durum, f, ensure_ascii=False, indent=1, sort_keys=True)


def _kayit(girdi: str, args) -> dict:
    st = os.stat(girdi)
    return {"kaynak": os.path.abspath(girdi), "mtime": st.st_mtime,
            "boyut": st.st_size, "imza": _imza(args)}


def atlanabilir(girdi: str, ad: str, args, durum: dict) -> str:
    """Çıktı bu kaynaktan ve aynı seçeneklerle üretilmişse nedenini döndürür."""
    if args.zorla:
        return ""
    kayit = durum.get(_anahtar(args.hedef, ad))
    if not kayit:
        return ""
    mp4 = os.path.join(args.hedef, f"{ad}.mp4")
    jpg = os.path.join(args.hedef, f"{ad}.jpg")
    if not (os.path.exists(mp4) and os.path.exists(jpg)):
        return ""
    if kayit.get("kaynak") != os.path.abspath(girdi) or kayit.get("imza") != _imza(args):
        return ""
    try:
        st = os.stat(girdi)
    except OSError:
        return ""
    if kayit.get("boyut") != st.st_size or abs(float(kayit.get("mtime", -1)) - st.st_mtime) > 1e-6:
        return ""
    return ("kaynak değişmedi ("
            + time.strftime("%d.%m %H:%M", time.localtime(st.st_mtime)) + ")")


def ffprobe(yol: str) -> dict:
    """ffprobe çıktısını sözlük olarak döndürür; hataları kısa Türkçe mesaja çevirir."""
    sonuc = subprocess.run(
        ["ffprobe", "-v", "error", "-print_format", "json",
         "-show_format", "-show_streams", yol],
        capture_output=True, text=True,
    )
    ad = os.path.basename(yol)
    if sonuc.returncode != 0:
        satirlar = [s.strip() for s in (sonuc.stderr or "").splitlines() if s.strip()]
        ayrinti = satirlar[-1] if satirlar else "ffprobe dosyayı okuyamadı"
        if ayrinti.startswith(yol):          # "<yol>: sebep" → yalnızca sebep
            ayrinti = ayrinti[len(yol):].lstrip(": ")
        ayrinti = ayrinti[:160]
        raise RuntimeError(f"{ad}: video okunamadı — dosya bozuk ya da video değil ({ayrinti})")
    try:
        return json.loads(sonuc.stdout)
    except ValueError:
        raise RuntimeError(f"{ad}: video bilgisi çözülemedi (ffprobe çıktısı geçersiz)")


def calistir(komut: list) -> None:
    sonuc = subprocess.run(komut, capture_output=True, text=True)
    if sonuc.returncode != 0:
        raise RuntimeError(sonuc.stderr.strip()[-600:] or "ffmpeg hatası")


def video_akisi(yol: str) -> dict:
    v = ffprobe(yol)
    akis = next((s for s in v["streams"] if s["codec_type"] == "video"), None)
    if akis is None:
        raise SystemExit("girdide video akışı yok")
    return akis


def bilgi(yol: str) -> str:
    v = ffprobe(yol)
    akis = next((s for s in v["streams"] if s["codec_type"] == "video"), None)
    if akis is None:
        raise RuntimeError(f"{os.path.basename(yol)}: video akışı yok (ses-only dosya olabilir)")
    sure = float(v["format"].get("duration") or 0)
    donuk = akis.get("tags", {}).get("rotate")
    if not donuk:
        yan = next((d for d in akis.get("side_data_list", []) if "rotation" in d), None)
        donuk = yan.get("rotation") if yan else None
    return (f"{akis['width']}x{akis['height']} · {sure:.1f} sn · "
            f"{akis.get('r_frame_rate', '?')} fps"
            + (f" · döndürme {donuk}°" if donuk else ""))


def _profiller(girdi: str, baslangic: float, sure: float, adet: int = 12):
    """Küçük karelerden satır/sütun detay profillerini çıkarır."""
    from PIL import Image

    gecici = tempfile.mkdtemp(prefix="klip_")
    try:
        calistir([
            "ffmpeg", "-y", "-loglevel", "error",
            "-ss", str(baslangic), "-t", str(sure), "-i", girdi,
            "-vf", f"fps={adet / max(sure, .1):.4f},scale=120:-2",
            "-q:v", "4", os.path.join(gecici, "%03d.jpg"),
        ])
        dosyalar = sorted(f for f in os.listdir(gecici) if f.endswith(".jpg"))
        if not dosyalar:
            return None, None, 0
        satir_top, sutun_top, adet_kare = None, None, 0
        for ad in dosyalar:
            g = Image.open(os.path.join(gecici, ad)).convert("L")
            w, h = g.size
            veri = g.tobytes()
            satirlar, sutunlar = [], []
            for y in range(h):
                s = veri[y * w:(y + 1) * w]
                m = sum(s) / w
                satirlar.append((sum((p - m) ** 2 for p in s) / w) ** .5)
            for x in range(w):
                s = veri[x::w]
                m = sum(s) / h
                sutunlar.append((sum((p - m) ** 2 for p in s) / h) ** .5)
            if satir_top is None:
                satir_top = [0.0] * h
                sutun_top = [0.0] * w
            for i in range(h):
                satir_top[i] += satirlar[i]
            for i in range(w):
                sutun_top[i] += sutunlar[i]
            adet_kare += 1
        return ([v / adet_kare for v in satir_top],
                [v / adet_kare for v in sutun_top], adet_kare)
    finally:
        shutil.rmtree(gecici, ignore_errors=True)


def _pencere(profil: list, boy: int, adim: int = 2):
    """Profilde en yüksek enerjili pencereyi ve tüm pencere puanlarını döndürür."""
    if boy >= len(profil):
        return 0, {0: sum(profil)}
    puanlar = {}
    for o in range(0, len(profil) - boy + 1, adim):
        puanlar[o] = sum(profil[o:o + boy])
    en = max(puanlar, key=puanlar.get)
    return en, puanlar


def kirpma_karari(girdi: str, baslangic: float, sure: float, kip: str = "akilli"):
    """(filtre, aciklama) döndürür. Kare oranı ne olursa olsun 16:9'a kırpar."""
    akis = video_akisi(girdi)
    # ffmpeg otomatik döndürme uygular; ölçüleri görüntü uzayında al.
    w, h = akis["width"], akis["height"]
    donuk = akis.get("tags", {}).get("rotate")
    if not donuk:
        yan = next((d for d in akis.get("side_data_list", []) if "rotation" in d), None)
        donuk = abs(int(float(yan["rotation"]))) if yan else 0
    if donuk % 180 == 90:
        w, h = h, w

    # yatay kaynak: genişlikten kırpılır, dikey kaynak: yükseklikten
    if w / h > HEDEF_ORAN:
        kes = min(round(h * HEDEF_ORAN), w)
        kay = (w - kes) / 2 / w
        return (f"crop={kes}:ih:trunc({kay:.5f}*iw):0",
                f"merkez · yatay kırpma {kes}px (kaynak {w}x{h})")

    kes = min(round(w / HEDEF_ORAN), h)          # kaynak piksellerinde kırpma yüksekliği
    satir = None
    if kip in ("akilli", "ust-orta"):
        satir, _, _ = _profiller(girdi, baslangic, sure)

    def merkez_filtresi() -> tuple:
        kay = (h - kes) / 2 / h
        return (f"crop=iw:{kes}:0:trunc({kay:.5f}*ih)",
                f"merkez · dikey kırpma {kes}px (kaynak {w}x{h})")

    if kip == "merkez" or not satir:
        return merkez_filtresi()

    # profil küçük karelerden gelir: pencere yüksekliğini aynı ölçeğe çevir
    n = len(satir)
    kes_p = max(1, min(n, round(kes * n / h)))
    _, puanlar = _pencere(satir, kes_p)
    # Aynı puanlı pencereler "plato" oluşturur (konu tamamen içeride olduğu sürece
    # puan değişmez). Bu yüzden en iyilerin tümünü alıp kareye göre en oturaklı
    # olanı seçeriz; yoksa çerçeve konunun üstüne kayar.
    esik = max(puanlar.values()) * .98
    adaylar = [o for o, p in puanlar.items() if p >= esik]
    merkez_p = (n - kes_p) / 2
    merkeze_yakin = min(adaylar, key=lambda o: abs(o - merkez_p))
    mod = kip
    if kip == "akilli":
        mod = "merkez" if abs(merkeze_yakin - merkez_p) <= .04 * n else "ust-orta"
    if mod == "merkez":
        return merkez_filtresi()

    # üst-orta: pencere merkezi karenin %14–%52 bandında kalsın, hedef %30
    hedef = .30 * n
    uygun = [o for o in adaylar if .14 * n <= o + kes_p / 2 <= .52 * n] or adaylar
    en = min(uygun, key=lambda o: abs(o + kes_p / 2 - hedef))
    kay = min(max(0.0, en / n), (h - kes) / h)
    return (f"crop=iw:{kes}:0:trunc({kay:.5f}*ih)",
            f"üst-orta · konu bandı y≈%{100 * kay:.0f}, kırpma {kes}px (kaynak {w}x{h})")


def filtre(sure: float, gecis: float, kirp: str) -> str:
    """Kuyruğu başlangıçla harmanlayıp dikişsiz döngü üretir."""
    a = round(sure - gecis, 3)
    return (
        f"[0:v]trim=0:{a},setpts=PTS-STARTPTS[a];"
        f"[0:v]trim={a}:{sure},setpts=PTS-STARTPTS[b];"
        f"[0:v]trim=0:{gecis},setpts=PTS-STARTPTS[c];"
        f"[b][c]blend=all_expr='A*(1-(T/{gecis}))+B*(T/{gecis})'[x];"
        f"[a][x]concat=n=2:v=1:a=0,fps=24,{kirp},"
        f"scale=960:540:force_original_aspect_ratio=increase,"
        f"crop=960:540,setsar=1,format=yuv420p[v]"
    )


def klip_uret(girdi: str, ad: str, args) -> dict:
    """Tek çekimi klibe çevirir; özet sözlüğü döndürür."""
    os.makedirs(args.hedef, exist_ok=True)
    mp4 = os.path.join(args.hedef, f"{ad}.mp4")
    jpg = os.path.join(args.hedef, f"{ad}.jpg")

    kaynak = bilgi(girdi)
    kirp, aciklama = kirpma_karari(girdi, args.baslangic, args.sure, args.kirpma)
    print(f"[{ad}] girdi: {kaynak}")
    print(f"[{ad}] kırpma: {aciklama}")

    dongu = not args.klavuzsuz and args.gecis > 0
    if dongu:
        try:
            calistir([
                "ffmpeg", "-y", "-loglevel", "error",
                "-ss", str(args.baslangic), "-t", str(args.sure + 1), "-i", girdi,
                "-filter_complex", filtre(args.sure, min(args.gecis, args.sure / 3), kirp),
                "-map", "[v]", "-c:v", "libx264", "-preset", "slow", "-crf", str(args.crf),
                "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an", mp4,
            ])
        except RuntimeError as e:
            print(f"[{ad}] çapraz geçiş yapılamadı, düz kesim:", str(e)[:160])
            dongu = False
    if not dongu:
        calistir([
            "ffmpeg", "-y", "-loglevel", "error",
            "-ss", str(args.baslangic), "-t", str(args.sure), "-i", girdi,
            "-vf", f"fps=24,{kirp},scale=960:540:force_original_aspect_ratio=increase,"
                   "crop=960:540,setsar=1,format=yuv420p",
            "-c:v", "libx264", "-preset", "slow", "-crf", str(args.crf),
            "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an", mp4,
        ])

    calistir([
        "ffmpeg", "-y", "-loglevel", "error", "-ss", str(args.sure / 2), "-i", mp4,
        "-frames:v", "1", "-vf", "scale=960:540", "-q:v", "3", jpg,
    ])

    boyut = os.path.getsize(mp4)
    sure_cikti = float(ffprobe(mp4)["format"]["duration"])
    print(f"[{ad}] çıktı: {sure_cikti:.2f} sn · {boyut / 1024:.0f} KB · {bilgi(mp4)}")
    if boyut > 300 * 1024:
        print(f"[{ad}] uyarı: 300 KB üstü — --crf değerini artır (ör. 33)")
    return {"ad": ad, "kb": boyut / 1024, "sure": sure_cikti, "kirpma": aciklama,
            "dongu": dongu, "mp4": mp4, "jpg": jpg}


def klip_listesine_ekle(adlar, kuru: bool = False) -> list:
    """site/anasayfa.html içindeki `klipler` dizisine eksik klipleri ekler.

    Yalnızca dosyası gerçekten var olan (mp4 + jpg) klipler eklenir; böylece
    ana sayfa 404 video göstermez. İşlem tekrarlanabilir (idempotent).
    """
    sayfa = os.path.join("site", "anasayfa.html")
    if not os.path.exists(sayfa):
        print("ana sayfa bulunamadı:", sayfa)
        return []
    with open(sayfa, encoding="utf-8") as f:
        metin = f.read()

    bas = metin.find("var klipler = [")
    kapanis = metin.find("];", bas + 1) if bas >= 0 else -1
    if bas < 0 or kapanis < 0:
        print("ana sayfada `klipler` dizisi bulunamadı")
        return []

    dizi = metin[bas:kapanis]
    mevcut = set(re.findall(r"video/hero/([A-Za-z0-9_-]+)\.mp4", dizi))
    eksik, atlanan = [], []
    for ad in dict.fromkeys(adlar):
        if ad in mevcut:
            continue
        if not re.fullmatch(r"[A-Za-z0-9_-]+", ad):
            atlanan.append((ad, "geçersiz ad"))
            continue
        if not (os.path.exists(os.path.join(HEDEF_KLASOR, f"{ad}.mp4"))
                and os.path.exists(os.path.join(HEDEF_KLASOR, f"{ad}.jpg"))):
            atlanan.append((ad, "dosya yok"))
            continue
        eksik.append(ad)
        mevcut.add(ad)

    if not eksik:
        print(f"ana sayfa listesi: yeni klip yok (liste {len(mevcut)} klip)")
        for ad, neden in atlanan:
            print(f"  atlandı: {ad} ({neden})")
        return []

    govde = dizi.rstrip()
    if not govde.endswith(","):
        govde += ","
    for ad in eksik:
        govde += f"\n    {{ v: 'video/hero/{ad}.mp4', p: 'video/hero/{ad}.jpg' }}"
    yeni = metin[:bas] + govde + "\n  " + metin[kapanis:]

    if kuru:
        print("ana sayfa listesi (kuru çalışma, dosya yazılmadı): +" + ", ".join(eksik))
        return eksik
    with open(sayfa, "w", encoding="utf-8") as f:
        f.write(yeni)
    print("ana sayfa listesine eklendi:", ", ".join(eksik))
    return eksik


def yayinla(dogrula: bool = True) -> int:
    """Paketler + surge aynasına yükler, sonra canlıda oynadığını doğrular."""
    print("\n=== yayın hazırlanıyor ===", flush=True)
    sonuc = subprocess.run(
        [sys.executable, "araclar/surge_hazirla.py", "--yayinla"], check=False
    )
    if sonuc.returncode != 0:
        print("surge yayını başarısız (çıkış kodu", sonuc.returncode, ")")
        return sonuc.returncode
    if dogrula:
        print("\n=== canlı kontrol ===", flush=True)
        denetim = subprocess.run(
            [sys.executable, "araclar/canli_dogrula.py", "--surge"], check=False
        )
        if denetim.returncode != 0:
            print("canlı kontrol başarısız — klip adreste oynatılamıyor olabilir")
            return denetim.returncode
    print("freebuff.page için: yayin/deploy.json manifestini yayınla "
          "(Sites panosu veya ajan)")
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description="Telefon çekimini hero klibine çevirir.")
    p.add_argument("girdi", help="ham video dosyası ya da çekimlerin bulunduğu klasör")
    p.add_argument("--ad", help="çıktı adı (klasör modunda ön ek olur)")
    p.add_argument("--hedef", default=HEDEF_KLASOR, help=f"çıktı klasörü (varsayılan: {HEDEF_KLASOR})")
    p.add_argument("--baslangic", type=float, default=0, help="kaçıncı saniyeden başlasın")
    p.add_argument("--sure", type=float, default=6.0, help="klip süresi (sn)")
    p.add_argument("--gecis", type=float, default=.5, help="döngü çapraz geçişi (sn)")
    p.add_argument("--crf", type=int, default=30, help="sıkıştırma kalitesi (küçük = daha iyi)")
    p.add_argument("--kirpma", choices=("akilli", "merkez", "ust-orta"), default="akilli",
                   help="akilli: konum algıla (varsayılan)")
    p.add_argument("--klavuzsuz", action="store_true", help="çapraz geçişi kapat")
    p.add_argument("--yayinla", action="store_true", help="bitince iki adrese yayınla")
    p.add_argument("--baglama-yok", action="store_true",
                   help="ana sayfanın klip listesini güncelleme")
    p.add_argument("--kuru", action="store_true", help="dosya yazmadan ne yapılacağını göster")
    p.add_argument("--zorla", action="store_true",
                   help="değişmemiş çekimleri de yeniden üret")
    p.add_argument("--dogrula-yok", action="store_true",
                   help="yayından sonra canlı kontrolü atla")
    args = p.parse_args()

    if not os.path.exists(args.girdi):
        raise SystemExit(f"yol yok: {args.girdi}")

    # --- klasör modu -----------------------------------------------------
    if os.path.isdir(args.girdi):
        dosyalar = sorted(f for f in os.listdir(args.girdi)
                          if f.lower().endswith(VIDEO_UZANTI))
        if not dosyalar:
            raise SystemExit(f"klasörde video yok: {args.girdi}")
        print(f"klasör modu: {len(dosyalar)} çekim · hedef {args.hedef}\n")
        sonuclar, hatalar = [], []
        durum = _durum_oku()
        degisti = False
        for dosya in dosyalar:
            yol = os.path.join(args.girdi, dosya)
            govde = os.path.splitext(dosya)[0]
            ad = f"{args.ad}_{govde}" if args.ad else govde
            neden = atlanabilir(yol, ad, args, durum)
            if neden:
                print(f"[{ad}] atlandı: {neden}\n")
                sonuclar.append({"ad": ad, "atlandi": neden})
                continue
            try:
                sonuclar.append(klip_uret(yol, ad, args))
                durum[_anahtar(args.hedef, ad)] = _kayit(yol, args)
                degisti = True
            except (RuntimeError, SystemExit) as e:
                hatalar.append((dosya, str(e)[:200]))
                print(f"[{dosya}] HATA: {str(e)[:200]}")
            print()
        if degisti:
            _durum_yaz(durum)
        if sonuclar:
            print("=== özet ===")
            print(f"{'klip':16}{'boyut':>9}{'süre':>7}  kırpma")
            for s in sonuclar:
                if s.get("atlandi"):
                    print(f"{s['ad']:16}{'—':>9}{'—':>7}  atlandı: {s['atlandi']}")
                else:
                    print(f"{s['ad']:16}{s['kb']:8.0f}K{s['sure']:6.2f}s  {s['kirpma']}")
            yapilan = [s for s in sonuclar if not s.get("atlandi")]
            atlanan = len(sonuclar) - len(yapilan)
            toplam = sum(s["kb"] for s in yapilan)
            ek = f" · {atlanan} atlandı" if atlanan else ""
            print(f"toplam: {len(yapilan)} klip{ek} · {toplam / 1024:.2f} MB")
        if hatalar:
            print(f"\n{len(hatalar)} çekim dönüştürülemedi:")
            for dosya, hata in hatalar:
                print(f"  - {dosya}: {hata[:120]}")
        if sonuclar and not args.baglama_yok:
            print()
            klip_listesine_ekle([s["ad"] for s in sonuclar], kuru=args.kuru)
        if args.yayinla:
            yayinla(dogrula=not args.dogrula_yok)
        return 1 if hatalar else 0

    # --- tek dosya modu --------------------------------------------------
    ad = args.ad or os.path.splitext(os.path.basename(args.girdi))[0]
    durum = _durum_oku()
    neden = atlanabilir(args.girdi, ad, args, durum)
    if neden:
        print(f"[{ad}] atlandı: {neden}")
        sonuc = {"ad": ad, "atlandi": neden}
    else:
        try:
            sonuc = klip_uret(args.girdi, ad, args)
        except RuntimeError as e:
            print("HATA:", str(e)[:300])
            return 1
        durum[_anahtar(args.hedef, ad)] = _kayit(args.girdi, args)
        _durum_yaz(durum)
    if not args.baglama_yok:
        print()
        klip_listesine_ekle([sonuc["ad"]], kuru=args.kuru)
    if args.yayinla:
        yayinla(dogrula=not args.dogrula_yok)
    else:
        print("yayınlama: python araclar/surge_hazirla.py --yayinla")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print("\nkesildi")
        raise SystemExit(130)
    except (RuntimeError, OSError) as e:
        print("HATA:", str(e)[:300])
        raise SystemExit(1)
