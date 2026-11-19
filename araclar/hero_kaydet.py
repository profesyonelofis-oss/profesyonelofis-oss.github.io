"""Hero sahnelerini yerel tarayıcıda (three.js) render edip kareleri diske yazar.

Sunucu: http://127.0.0.1:8788/  -> aracilar/hero_sahne.html
Sayfa dört sahneyi (koridor, gunduz, gece, yagmur) sırayla çizer ve her kareyi
POST /kare ile gönderir; iş bitince /bitti çağrılır.
"""
import http.server
import os
import urllib.parse

KOK = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GEC = os.path.join(KOK, "_gecici", "hero_kareler")
PORT = 8788


class Isleyici(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **kw):
        super().__init__(*a, directory=KOK, **kw)

    def do_GET(self):
        if urllib.parse.urlparse(self.path).path in ("/", "/index.html"):
            yol = os.path.join(KOK, "araclar", "hero_sahne.html")
            with open(yol, "rb") as f:
                govde = f.read()
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(govde)))
            self.end_headers()
            self.wfile.write(govde)
            return
        super().do_GET()

    def do_POST(self):
        u = urllib.parse.urlparse(self.path)
        q = urllib.parse.parse_qs(u.query)
        n = int(self.headers.get("Content-Length", 0))
        veri = self.rfile.read(n)
        if u.path == "/kare":
            ad = q["ad"][0]
            i = int(q["i"][0])
            d = os.path.join(GEC, ad)
            os.makedirs(d, exist_ok=True)
            with open(os.path.join(d, f"{i:04d}.jpg"), "wb") as f:
                f.write(veri)
        elif u.path == "/bitti":
            os.makedirs(GEC, exist_ok=True)
            with open(os.path.join(GEC, "BITTI"), "w", encoding="utf-8") as f:
                f.write(q.get("ad", [""])[0])
        self.send_response(204)
        self.end_headers()

    def log_message(self, *a):
        pass


if __name__ == "__main__":
    os.makedirs(GEC, exist_ok=True)
    kes = os.path.join(GEC, "BITTI")
    if os.path.exists(kes):
        os.remove(kes)
    with http.server.ThreadingHTTPServer(("127.0.0.1", PORT), Isleyici) as s:
        print(f"hero kaydedici hazır: http://127.0.0.1:{PORT}/", flush=True)
        s.serve_forever()
