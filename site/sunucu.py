#!/usr/bin/env python3
"""Saruhanlı Oto Kurtarma — yerel önizleme sunucusu.

`site/` klasörünü kök kabul eder. Var olmayan bir yol istendiğinde 404
yerine ana sayfayı döndürür; böylece /site/index.html, /anasayfa gibi
adresler de çalışır.

Kullanım:  python site/sunucu.py --port 8907
"""

import argparse
import os
import posixpath
import shutil
import sys
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer

# Windows konsolu cp1254 olabiliyor; loglar UTF-8'e zorlanır.
for akis in (sys.stdout, sys.stderr):
    try:
        akis.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, ValueError):
        pass

KOK = os.path.dirname(os.path.abspath(__file__))

# Windows'ta bazı sürümler bu tipleri tanımıyor; açıkça veriyoruz.
SimpleHTTPRequestHandler.extensions_map.update(
    {
        ".woff2": "font/woff2",
        ".woff": "font/woff",
        ".mp4": "video/mp4",
        ".webp": "image/webp",
        ".avif": "image/avif",
        ".json": "application/json",
        ".webmanifest": "application/manifest+json",
    }
)


class Sunucu(SimpleHTTPRequestHandler):
    """Kök `site/`; bilinmeyen yollar ana sayfaya düşer, Range desteklidir."""

    ONEKLER = ("/site", "/public", "/dist")
    _kalan = None

    def translate_path(self, path):
        temiz = path.split("?", 1)[0].split("#", 1)[0]
        temiz = posixpath.normpath(temiz) if temiz.strip("/") else "/"
        for onek in self.ONEKLER:
            if temiz == onek or temiz.startswith(onek + "/"):
                temiz = temiz[len(onek):] or "/"
                break
        return super().translate_path(temiz)

    def send_head(self):
        yol = self.translate_path(self.path)
        if not os.path.exists(yol) and not self.path.startswith("/video/"):
            self.path = "/index.html"
            yol = self.translate_path(self.path)
        if os.path.isdir(yol):
            return super().send_head()
        try:
            dosya = open(yol, "rb")
        except OSError:
            return super().send_head()

        boyut = os.fstat(dosya.fileno()).st_size
        son_degisim = self.date_time_string(os.fstat(dosya.fileno()).st_mtime)
        istek = self.headers.get("Range", "")
        bas = son = None
        if istek.startswith("bytes=") and boyut:
            parca = istek[6:].split(",")[0].strip()
            ilk, _, ikinci = parca.partition("-")
            try:
                if ilk:
                    bas = max(0, int(ilk))
                    son = int(ikinci) if ikinci else boyut - 1
                elif ikinci:
                    bas = max(0, boyut - int(ikinci))
                    son = boyut - 1
            except ValueError:
                bas = son = None

        if bas is not None and bas < boyut:
            son = min(son if son is not None else boyut - 1, boyut - 1)
            dosya.seek(bas)
            self._kalan = son - bas + 1
            self.send_response(206)
            self.send_header("Content-Type", self.guess_type(yol))
            self.send_header("Accept-Ranges", "bytes")
            self.send_header("Content-Range", f"bytes {bas}-{son}/{boyut}")
            self.send_header("Content-Length", str(self._kalan))
            self.send_header("Last-Modified", son_degisim)
            self.end_headers()
            return dosya

        self._kalan = None
        self.send_response(200)
        self.send_header("Content-Type", self.guess_type(yol))
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Length", str(boyut))
        self.send_header("Last-Modified", son_degisim)
        self.end_headers()
        return dosya

    def copyfile(self, kaynak, hedef):
        if self._kalan:
            shutil.copyfileobj(kaynak, hedef, self._kalan)
        else:
            shutil.copyfileobj(kaynak, hedef)
        self._kalan = None

    def log_message(self, kalip, *args):
        print("%s — %s" % (self.address_string(), kalip % args), flush=True)


def main():
    ap = argparse.ArgumentParser(description="Saruhanlı Oto Kurtarma yerel sunucu")
    ap.add_argument("--port", type=int, default=8907)
    ap.add_argument("--host", default="127.0.0.1")
    secim = ap.parse_args()

    isleyici = partial(Sunucu, directory=KOK)
    with ThreadingHTTPServer((secim.host, secim.port), isleyici) as sunucu:
        print(f"site hazir -> http://{secim.host}:{secim.port}/", flush=True)
        try:
            sunucu.serve_forever()
        except KeyboardInterrupt:
            print("\nkapatıldı", flush=True)


if __name__ == "__main__":
    main()
