"""Yerel önizleme için Range destekli basit HTTP sunucusu.

Python'un standart `http.server` modülü `Range` başlığını desteklemediği için
tarayıcı video içinde ileri-geri atlayamıyor (scroll-scrub çalışmıyor).
Bu betik yalnız geliştirme önizlemesi içindir; üretimde GitHub Pages Range
destekler.
"""

import os
import re
import sys
from http.server import HTTPServer, SimpleHTTPRequestHandler


class RangeHandler(SimpleHTTPRequestHandler):
    def send_head(self):
        yol = self.translate_path(self.path)
        if os.path.isdir(yol):
            return super().send_head()
        if not os.path.isfile(yol):
            return super().send_head()
        boyut = os.path.getsize(yol)
        aralik = self.headers.get("Range")
        if not aralik:
            return super().send_head()
        eslesme = re.match(r"bytes=(\d*)-(\d*)", aralik)
        if not eslesme:
            return super().send_head()
        bas = int(eslesme.group(1) or 0)
        son = int(eslesme.group(2)) if eslesme.group(2) else boyut - 1
        son = min(son, boyut - 1)
        if bas > son:
            self.send_error(416)
            return None
        dosya = open(yol, "rb")
        dosya.seek(bas)
        self.send_response(206)
        self.send_header("Content-Type", self.guess_type(yol))
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Range", f"bytes {bas}-{son}/{boyut}")
        self.send_header("Content-Length", str(son - bas + 1))
        self.end_headers()
        kalan = son - bas + 1

        class Sinirli:
            def __init__(self, f, n):
                self.f = f
                self.n = n

            def read(self, adet=-1):
                if self.n <= 0:
                    return b""
                adet = self.n if adet < 0 else min(adet, self.n)
                veri = self.f.read(adet)
                self.n -= len(veri)
                return veri

            def close(self):
                self.f.close()

        return Sinirli(dosya, kalan)

    def end_headers(self):
        self.send_header("Accept-Ranges", "bytes")
        super().end_headers()


if __name__ == "__main__":
    kok = sys.argv[1] if len(sys.argv) > 1 else "."
    port = int(sys.argv[2]) if len(sys.argv) > 2 else 8791
    os.chdir(kok)
    HTTPServer(("127.0.0.1", port), RangeHandler).serve_forever()
