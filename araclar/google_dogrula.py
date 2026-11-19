"""Google Search Console doğrulamasını yayın klasörüne ekler.

Kullanım: python araclar/google_dogrula.py <dogrulama-kodu>
  - <dogrulama-kodu>: HTML etiketindeki content değeri ya da dosya adındaki kod
    (ör. "AbC123..." veya "google1234abcd.html")

Yaptığı:
  1) yayin/public/google<kod>.html dosyasını yazar (HTML dosyası doğrulaması için)
  2) tüm HTML sayfalarının <head> bölümüne <meta name="google-site-verification"> ekler
     (HTML etiketi doğrulaması için), varsa tekrar eklemez.
"""
import glob
import os
import sys

KOK = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PUBLIC = os.path.join(KOK, "yayin", "public")
ETIKET = '<meta name="google-site-verification" content="{kod}">'


def main() -> int:
    if len(sys.argv) < 2:
        print("kullanım: python araclar/google_dogrula.py <dogrulama-kodu>")
        return 2
    kod = sys.argv[1].strip()
    if kod.startswith("google") and kod.endswith(".html"):
        kod = kod[len("google") : -len(".html")]
    if not kod:
        print("kod boş")
        return 2
    if not os.path.isdir(PUBLIC):
        print("önce yayin_hazirla.py çalıştırılmalı (yayin/public yok)")
        return 1

    dosya = os.path.join(PUBLIC, f"google{kod}.html")
    with open(dosya, "w", encoding="utf-8") as f:
        f.write(f"google-site-verification: google{kod}.html\n")

    eklenen = 0
    for yol in glob.glob(os.path.join(PUBLIC, "*.html")):
        with open(yol, encoding="utf-8") as f:
            s = f.read()
        if "google-site-verification" in s:
            continue
        etiket = ETIKET.format(kod=kod)
        if "</head>" in s:
            s = s.replace("</head>", etiket + "\n</head>", 1)
        else:
            s = etiket + "\n" + s
        with open(yol, "w", encoding="utf-8") as f:
            f.write(s)
        eklenen += 1

    uygun = sum(
        1
        for yol in glob.glob(os.path.join(PUBLIC, "*.html"))
        if "google-site-verification" in open(yol, encoding="utf-8").read()
    )
    print(f"doğrulama dosyası: {os.path.basename(dosya)}")
    print(f"meta etiketi eklenen sayfa: {eklenen} · toplam etiketli sayfa: {uygun}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
